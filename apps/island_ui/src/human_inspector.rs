//! The person inspector (Phase 5 Task 4): what the island says about one
//! person, section by section.
//!
//! The backend builds the sections -- identity, body, vital signs, needs,
//! temperament, feeling, attention and thinking, what they are doing and
//! what they are thinking, other people, growing up, family, their record
//! -- from the person's live state (`island::serve::view`), and the web
//! dashboard shows the same ones. This reads them off the wire rather
//! than rebuilding them from the engine's types: the wire is the contract
//! (`mk_island_client::IslandClient::human`), and two frontends that
//! each assembled a person their own way would disagree about them.
//!
//! Pure, so it is testable without a window.

use serde::Deserialize;

/// One reading: a label, its value in words, an optional sentence on what
/// it means, and a 0-1 meter where the value is a proportion.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Reading {
    pub label: String,
    pub value: String,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub meter: Option<f64>,
}

/// A section, as the backend titles and describes it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Section {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub blurb: String,
    #[serde(rename = "fields", default)]
    pub readings: Vec<Reading>,
}

/// Where the person is now, which only the running island knows.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Whereabouts {
    #[serde(default)]
    pub space: Option<String>,
    #[serde(default)]
    pub asleep: bool,
    #[serde(default)]
    pub body_carbon_kg: Option<f64>,
}

/// Everything the inspector shows about one person.
#[derive(Debug, Clone, PartialEq)]
pub struct Person {
    pub sections: Vec<Section>,
    pub whereabouts: Option<Whereabouts>,
    /// The tick the person's record was read at, and the island's tick
    /// when it was served: the record is refreshed hourly, so the two can
    /// differ and the inspector says by how much.
    pub records_at_tick: Option<u64>,
    pub tick: Option<u64>,
}

/// Read a `/api/world/humans/{id}` reply.
///
/// A reply without sections is refused rather than shown as an empty
/// person: it is a backend that does not serve them, and an inspector
/// that showed nothing would look like a person with nothing to say.
pub fn read(reply: &serde_json::Value) -> Result<Person, String> {
    let sections = reply
        .get("sections")
        .ok_or("this backend serves no sections for a person")?;
    let sections: Vec<Section> = serde_json::from_value(sections.clone())
        .map_err(|e| format!("the person's sections could not be read: {e}"))?;
    let mut section_ids = std::collections::HashSet::new();
    for section in &sections {
        if section.id.trim().is_empty() || !section_ids.insert(section.id.as_str()) {
            return Err("person sections have an empty or duplicate id".to_string());
        }
        for reading in &section.readings {
            if let Some(meter) = reading.meter {
                if !meter.is_finite() || !(0.0..=1.0).contains(&meter) {
                    return Err(format!("invalid meter in section {}", section.id));
                }
            }
        }
    }
    if sections.is_empty() {
        return Err("this backend served a person with no sections".to_string());
    }
    // A malformed live location is a wire-contract error, not offline state.
    let whereabouts: Option<Whereabouts> = reply
        .get("where")
        .filter(|w| !w.is_null())
        .map(|w| {
            serde_json::from_value(w.clone())
                .map_err(|e| format!("the person's whereabouts could not be read: {e}"))
        })
        .transpose()?;
    if let Some(ref location) = whereabouts {
        if location
            .body_carbon_kg
            .is_some_and(|kg| !kg.is_finite() || kg < 0.0)
        {
            return Err("invalid whereabouts body_carbon_kg".to_string());
        }
    }
    // Present-but-invalid clocks must not silently look like absent clocks.
    let read_tick = |key: &str| -> Result<Option<u64>, String> {
        match reply.get(key) {
            None | Some(serde_json::Value::Null) => Ok(None),
            Some(value) => value
                .as_u64()
                .map(Some)
                .ok_or_else(|| format!("invalid {key}: expected a nonnegative integer")),
        }
    };
    let records_at_tick = read_tick("records_at_tick")?;
    let tick = read_tick("tick")?;
    if matches!((records_at_tick, tick), (Some(record), Some(now)) if record > now) {
        return Err("records_at_tick exceeds island tick".to_string());
    }
    Ok(Person {
        sections,
        whereabouts,
        records_at_tick,
        tick,
    })
}

/// How old the reading is, in words, when it is older than the island.
pub fn freshness(person: &Person) -> Option<String> {
    let (read_at, now) = (person.records_at_tick?, person.tick?);
    let behind = now.saturating_sub(read_at);
    (behind > 0).then(|| {
        format!(
            "Read at tick {read_at}, {behind} tick{} ago: the island refreshes a person's record \
             every hour, and on creation.",
            if behind == 1 { "" } else { "s" }
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reply() -> serde_json::Value {
        serde_json::json!({
            "summary": { "agent_id": "Gem-D" },
            "sections": [
                {
                    "id": "needs",
                    "title": "How they are doing",
                    "blurb": "Hunger, thirst and tiredness.",
                    "fields": [
                        { "label": "Hunger", "value": "12%", "meter": 0.12 },
                        { "label": "Survival", "value": "Fine", "detail": "Nothing is failing." }
                    ]
                },
                {
                    "id": "family",
                    "title": "Family",
                    "blurb": "Their parents and their children.",
                    "fields": [ { "label": "Children", "value": "None" } ]
                }
            ],
            "where": { "space": "Gem-D's Bedroom", "asleep": true, "body_carbon_kg": 13.0 },
            "records_at_tick": 120,
            "tick": 150
        })
    }

    #[test]
    fn duplicate_and_empty_section_ids_are_rejected() {
        let mut duplicate = reply();
        duplicate["sections"][1]["id"] = serde_json::json!("needs");
        assert!(read(&duplicate).unwrap_err().contains("duplicate id"));
        let mut empty = reply();
        empty["sections"][0]["id"] = serde_json::json!("  ");
        assert!(read(&empty).unwrap_err().contains("empty or duplicate id"));
    }

    #[test]
    fn future_record_ticks_are_rejected_instead_of_appearing_fresh() {
        let mut payload = reply();
        payload["records_at_tick"] = serde_json::json!(151);
        assert!(read(&payload)
            .unwrap_err()
            .contains("records_at_tick exceeds island tick"));
    }

    #[test]
    fn the_sections_are_the_backends_in_its_order_and_its_words() {
        let person = read(&reply()).unwrap();
        let titles: Vec<&str> = person.sections.iter().map(|s| s.title.as_str()).collect();
        assert_eq!(titles, ["How they are doing", "Family"]);
        let hunger = &person.sections[0].readings[0];
        assert_eq!((hunger.value.as_str(), hunger.meter), ("12%", Some(0.12)));
        assert_eq!(
            person.sections[0].readings[1].detail.as_deref(),
            Some("Nothing is failing.")
        );
    }

    #[test]
    fn where_they_are_comes_from_the_running_island() {
        let person = read(&reply()).unwrap();
        let here = person.whereabouts.unwrap();
        assert_eq!(here.space.as_deref(), Some("Gem-D's Bedroom"));
        assert!(here.asleep);
        // A stored population, with no island running, has no answer.
        let mut stored = reply();
        stored["where"] = serde_json::Value::Null;
        assert_eq!(read(&stored).unwrap().whereabouts, None);
    }

    #[test]
    fn malformed_live_whereabouts_are_rejected_not_shown_as_offline() {
        let mut corrupt = reply();
        corrupt["where"] = serde_json::json!({"asleep": "not a boolean"});
        assert!(read(&corrupt).unwrap_err().contains("whereabouts"));
    }

    #[test]
    fn invalid_live_carbon_is_a_wire_error() {
        let mut corrupt = reply();
        corrupt["where"] = serde_json::json!({"asleep": false, "body_carbon_kg": "unknown"});
        assert!(read(&corrupt).unwrap_err().contains("whereabouts"));
    }

    #[test]
    fn malformed_tick_fields_are_rejected_instead_of_hidden() {
        for (field, invalid) in [
            ("tick", serde_json::json!(-1)),
            ("records_at_tick", serde_json::json!("120")),
            ("tick", serde_json::json!(1.5)),
        ] {
            let mut corrupt = reply();
            corrupt[field] = invalid;
            assert!(read(&corrupt).unwrap_err().contains(field));
        }
    }

    #[test]
    fn invalid_meters_and_negative_carbon_are_rejected() {
        for invalid in [serde_json::json!(-0.1), serde_json::json!(1.01)] {
            let mut corrupt = reply();
            corrupt["sections"][0]["fields"][0]["meter"] = invalid;
            assert!(read(&corrupt).unwrap_err().contains("meter"));
        }
        let mut corrupt = reply();
        corrupt["where"]["body_carbon_kg"] = serde_json::json!(-0.5);
        assert!(read(&corrupt).unwrap_err().contains("body_carbon_kg"));
    }

    #[test]
    fn a_reading_older_than_the_island_says_so() {
        let person = read(&reply()).unwrap();
        let said = freshness(&person).unwrap();
        assert!(
            said.contains("tick 120") && said.contains("30 ticks ago"),
            "{said}"
        );
        let mut current = reply();
        current["records_at_tick"] = 150.into();
        assert_eq!(freshness(&read(&current).unwrap()), None);
    }

    #[test]
    fn a_reply_with_no_sections_is_refused_not_shown_empty() {
        assert!(read(&serde_json::json!({ "summary": {} })).is_err());
        assert!(read(&serde_json::json!({ "sections": [] })).is_err());
    }
}

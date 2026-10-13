//! The foundry (Phase 5 Task 4): somebody new, made from the desktop.
//!
//! Its fields are exactly the island's creation request
//! (`mk_island_api::CreateHumanRequest`) and its choices exactly what the
//! creator accepts (`/api/creator/options`), so a person made here is the
//! person the dashboard's creator page would make from the same answers:
//! both send the same request down the same route. Nothing a person is
//! *derived* to have -- temperament, neurotype, a birth chart -- is a
//! field here; those come out of the creation, and the person inspector
//! shows them once the person exists.
//!
//! Pure, so it is testable without a window.

use mk_island_api::{CreateHumanRequest, CreatorOptions};

/// Where the new person is put: a room of the estate, or a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    Space(u32),
    Cell(usize, usize),
}

/// What somebody has filled in so far.
#[derive(Debug, Clone, PartialEq)]
pub struct Form {
    pub name: String,
    pub biological_sex: String,
    /// The date and time of birth as a person types it,
    /// `YYYY-MM-DDTHH:MM` (or with seconds), in the zone of `birth_offset`.
    pub birth_local: String,
    /// `+13:00`, `-05:30`.
    pub birth_offset: String,
    pub age_years: f64,
    pub height_cm: f64,
    pub build: String,
    pub hair_color: String,
    pub eye_color: String,
    pub skin_tone: String,
    pub place: Option<Place>,
}

impl Form {
    /// A blank form: every choice at the first thing the creator offers,
    /// an adult's age and a height in the middle of what is allowed, and
    /// no name and no birth, which only the person filling it in can give.
    pub fn new(options: &CreatorOptions) -> Self {
        let first = |list: &[String]| list.first().cloned().unwrap_or_default();
        Self {
            name: String::new(),
            biological_sex: first(&options.biological_sex),
            birth_local: String::new(),
            birth_offset: "+00:00".to_string(),
            age_years: 25.0_f64.min(options.max_age_years),
            height_cm: 170.0_f64.clamp(options.height_cm.0, options.height_cm.1),
            build: first(&options.build),
            hair_color: first(&options.hair_color),
            eye_color: first(&options.eye_color),
            skin_tone: first(&options.skin_tone),
            place: None,
        }
    }

    /// The birth as RFC 3339, built the way the dashboard's creator builds
    /// it: seconds added when the time has none, then the offset.
    pub fn birth_timestamp(&self) -> String {
        let local = self.birth_local.trim();
        if local.is_empty() {
            return String::new();
        }
        let seconds = if local.len() == 16 { ":00" } else { "" };
        format!("{local}{seconds}{}", self.birth_offset.trim())
    }

    /// The request this form sends, or every reason it cannot be sent yet.
    ///
    /// Only what can be known here is checked: that each answer is given,
    /// is one the creator offers and is inside its limits. Whether the room
    /// still exists or the cell is land is the island's to say, and it
    /// says so in its answer.
    pub fn request(&self, options: &CreatorOptions) -> Result<CreateHumanRequest, Vec<String>> {
        let mut problems = Vec::new();
        let name = self.name.trim();
        if name.is_empty() {
            problems.push("They need a name.".to_string());
        }
        let birth = self.birth_timestamp();
        if birth.is_empty() {
            problems.push("They need a date and time of birth.".to_string());
        } else if !looks_like_a_local_time(self.birth_local.trim()) {
            problems
                .push("The date and time of birth should look like 1999-09-09T09:09.".to_string());
        }
        if !looks_like_an_offset(self.birth_offset.trim()) {
            problems.push("The UTC offset should look like +13:00 or -05:30.".to_string());
        }
        if !(0.0..=options.max_age_years).contains(&self.age_years) {
            problems.push(format!(
                "Their age should be between 0 and {} years.",
                options.max_age_years
            ));
        }
        let (short, tall) = options.height_cm;
        if !(short..=tall).contains(&self.height_cm) {
            problems.push(format!(
                "Their height should be between {short} and {tall} cm."
            ));
        }
        for (what, chosen, offered) in [
            ("sex", &self.biological_sex, &options.biological_sex),
            ("build", &self.build, &options.build),
            ("hair colour", &self.hair_color, &options.hair_color),
            ("eye colour", &self.eye_color, &options.eye_color),
            ("skin tone", &self.skin_tone, &options.skin_tone),
        ] {
            if !offered.contains(chosen) {
                problems.push(format!("`{chosen}` is not a {what} the creator offers."));
            }
        }
        if self.place.is_none() {
            problems.push("They need somewhere to be: a room of the estate or a cell.".to_string());
        }
        if !problems.is_empty() {
            return Err(problems);
        }
        let (space, row, col) = match self.place.expect("checked above") {
            Place::Space(id) => (Some(id), None, None),
            Place::Cell(row, col) => (None, Some(row), Some(col)),
        };
        Ok(CreateHumanRequest {
            name: name.to_string(),
            biological_sex: self.biological_sex.clone(),
            birth_timestamp: birth,
            age_years: self.age_years,
            height_cm: self.height_cm,
            build: self.build.clone(),
            hair_color: self.hair_color.clone(),
            eye_color: self.eye_color.clone(),
            skin_tone: self.skin_tone.clone(),
            space,
            row,
            col,
            // Born here, which is what the dashboard sends by leaving it
            // out: the request's own default.
            birthplace_here: true,
            birth_latitude: 0.0,
            birth_longitude: 0.0,
        })
    }
}

/// `YYYY-MM-DDTHH:MM`, with optional `:SS`.
fn looks_like_a_local_time(text: &str) -> bool {
    let shape = |pattern: &str| {
        text.len() == pattern.len()
            && text.chars().zip(pattern.chars()).all(|(c, p)| match p {
                'd' => c.is_ascii_digit(),
                other => c == other,
            })
    };
    shape("dddd-dd-ddTdd:dd") || shape("dddd-dd-ddTdd:dd:dd")
}

/// `+HH:MM` or `-HH:MM`.
fn looks_like_an_offset(text: &str) -> bool {
    let b = text.as_bytes();
    b.len() == 6
        && (b[0] == b'+' || b[0] == b'-')
        && b[1].is_ascii_digit()
        && b[2].is_ascii_digit()
        && b[3] == b':'
        && b[4].is_ascii_digit()
        && b[5].is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> CreatorOptions {
        let words = |w: &[&str]| w.iter().map(|s| s.to_string()).collect();
        CreatorOptions {
            biological_sex: words(&["female", "male"]),
            build: words(&["Slim", "Average"]),
            hair_color: words(&["Black", "Brown"]),
            eye_color: words(&["Brown", "Blue"]),
            skin_tone: words(&["Fair", "Olive"]),
            max_age_years: 130.0,
            height_cm: (40.0, 272.0),
        }
    }

    fn ata() -> Form {
        Form {
            name: " Ata ".to_string(),
            biological_sex: "female".to_string(),
            birth_local: "1999-09-09T09:09".to_string(),
            birth_offset: "+13:00".to_string(),
            age_years: 22.0,
            height_cm: 165.0,
            build: "Average".to_string(),
            hair_color: "Black".to_string(),
            eye_color: "Brown".to_string(),
            skin_tone: "Olive".to_string(),
            place: Some(Place::Space(7)),
        }
    }

    #[test]
    fn the_foundry_sends_what_the_dashboards_creator_sends() {
        // The body `creator.js` posts for the same answers, as JSON: the
        // name trimmed, seconds added to the time and the offset after it,
        // the room as `space`, and no birthplace sent, so it defaults.
        let dashboard: CreateHumanRequest = serde_json::from_value(serde_json::json!({
            "name": "Ata",
            "biological_sex": "female",
            "birth_timestamp": "1999-09-09T09:09:00+13:00",
            "age_years": 22,
            "height_cm": 165,
            "build": "Average",
            "hair_color": "Black",
            "eye_color": "Brown",
            "skin_tone": "Olive",
            "space": 7
        }))
        .unwrap();
        assert_eq!(ata().request(&options()).unwrap(), dashboard);
    }

    #[test]
    fn a_cell_is_sent_as_a_cell_and_never_with_a_room() {
        let mut form = ata();
        form.place = Some(Place::Cell(608, 553));
        let request = form.request(&options()).unwrap();
        assert_eq!(
            (request.space, request.row, request.col),
            (None, Some(608), Some(553))
        );
    }

    #[test]
    fn an_unfinished_form_says_everything_it_still_needs() {
        let blank = Form::new(&options());
        let problems = blank.request(&options()).unwrap_err();
        assert!(problems.iter().any(|p| p.contains("name")), "{problems:?}");
        assert!(problems.iter().any(|p| p.contains("birth")), "{problems:?}");
        assert!(
            problems.iter().any(|p| p.contains("somewhere")),
            "{problems:?}"
        );
    }

    #[test]
    fn answers_the_creator_would_refuse_are_caught_here() {
        let mut form = ata();
        form.age_years = 131.0;
        form.height_cm = 300.0;
        form.hair_color = "Green".to_string();
        form.birth_offset = "13:00".to_string();
        form.birth_local = "9 Sept 1999".to_string();
        let problems = form.request(&options()).unwrap_err();
        assert_eq!(problems.len(), 5, "{problems:?}");
    }

    #[test]
    fn a_time_with_seconds_keeps_them() {
        let mut form = ata();
        form.birth_local = "1999-09-09T09:09:30".to_string();
        assert_eq!(form.birth_timestamp(), "1999-09-09T09:09:30+13:00");
    }

    #[test]
    fn a_blank_form_starts_inside_every_limit() {
        let form = Form::new(&options());
        assert_eq!(form.biological_sex, "female");
        assert!((0.0..=130.0).contains(&form.age_years));
        assert!((40.0..=272.0).contains(&form.height_cm));
    }
}

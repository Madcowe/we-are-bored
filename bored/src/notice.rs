use crate::{Bored, BoredAddress, BoredError, Coordinate};
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Limit to avoid massive amount of text being accidentally put into hyperlink and making
/// bored to big to fit in scratchpadlonges
pub const MAX_URL_LENGTH: usize = 1024;

/// Hyperlinks with maximum url length
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Hyperlink {
    text: String,
    text_location: (usize, usize),
    link: String,
    link_location: (usize, usize),
}
impl Hyperlink {
    pub fn create(
        text: &str,
        text_location: (usize, usize),
        link: &str,
        link_location: (usize, usize),
    ) -> Result<Hyperlink, BoredError> {
        if link.len() > MAX_URL_LENGTH {
            return Err(BoredError::URLTooLong);
        }
        Ok(Hyperlink {
            text: text.to_string(),
            text_location,
            link: link.to_string(),
            link_location,
        })
    }
}

/// Display contains the text to display plus a collections of the hyperlinks locations from left
/// to right
pub struct Display {
    display_text: String,
    hyperlink_locations: Vec<(usize, usize)>,
}
impl Display {
    /// create new display with empty string and vector
    pub fn new() -> Display {
        Display {
            display_text: String::new(),
            hyperlink_locations: vec![],
        }
    }

    pub fn get_display_text(&self) -> String {
        self.display_text.clone()
    }

    /// Descrease every location value in hyperlinks verctor by
    /// This is so that they can be adjusted as the display string is being created
    pub fn decrement_hyperlink_locations(&mut self, decrease_by: usize) {
        for i in 0..self.hyperlink_locations.len() {
            if decrease_by <= self.hyperlink_locations[i].0 {
                self.hyperlink_locations[i].0 -= decrease_by;
            } else {
                self.hyperlink_locations[i].0 = 0;
            }
            if decrease_by <= self.hyperlink_locations[i].1 {
                self.hyperlink_locations[i].1 -= decrease_by;
            } else {
                self.hyperlink_locations[i].1 = 0;
            }
        }
    }
}

/// A notice the may be attached to a bored containing only as much text as would be visible
/// within it's bounds (not counting not visble parts of hyperlinks)
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Notice {
    top_left: Coordinate,
    dimensions: Coordinate, // the notice will range from (0,0) up to
    content: String,
}

impl Notice {
    /// Create a new blank note at (0,0)
    pub fn new() -> Notice {
        Notice {
            top_left: Coordinate { x: 0, y: 0 },
            dimensions: Coordinate { x: 60, y: 18 },
            content: String::new(),
        }
    }

    /// Create a new blank note of specfied size at (0,0)
    pub fn create(dimensions: Coordinate) -> Notice {
        Notice {
            top_left: Coordinate { x: 0, y: 0 },
            dimensions,
            content: String::new(),
        }
    }

    pub fn get_top_left(&self) -> Coordinate {
        self.top_left
    }

    pub fn get_dimensions(&self) -> Coordinate {
        self.dimensions
    }

    pub fn get_content(&self) -> &str {
        &self.content
    }

    /// moves notices position on board, both prior to placing and is called by Bored.add()
    pub fn relocate(&mut self, bored: &Bored, new_top_left: Coordinate) -> Result<(), BoredError> {
        let new_bottom_right = new_top_left.add(&self.dimensions);
        if new_bottom_right.within(&bored.dimensions) {
            self.top_left = new_top_left;
            return Ok(());
        }
        Err(BoredError::NoticeOutOfBounds)
    }

    /// Get maximun nubmer of unicode scarlar value that can be written on the notice
    // If you wanted to handle some other langauge you might need to work out hot to implement
    // for graphem clusters instead
    pub fn get_max_chars(&self) -> usize {
        let area = self.dimensions.x * self.dimensions.y;
        if area < 9 {
            // 3 * 3 is the smallest dimension with any space
            return 0;
        } else {
            // area minus border
            (area - ((2 * self.dimensions.x) + (2 * (self.dimensions.y - 2)))).into()
        }
    }

    /// Get number of lines that can be written on the notice
    pub fn get_max_lines(&self) -> usize {
        if self.dimensions.y < 2 {
            return 0;
        } else {
            (self.dimensions.y - 2).into()
        }
    }

    /// Add textual content to the notice, will only allow as much text and lines as will fit in
    pub fn write(&mut self, content: &str) -> Result<(), BoredError> {
        let display_text = get_display(&content, get_hyperlinks(content)?).display_text;
        if display_text.chars().count() > self.get_max_chars()
            || display_text.lines().count() > self.get_max_lines()
        {
            return Err(BoredError::TooMuchText);
        }
        self.content = content.to_string();
        Ok(())
    }
}

/// Returns a vector of all the hyperlinks in the text using markdown link notation
pub fn get_hyperlinks(content: &str) -> Result<Vec<Hyperlink>, BoredError> {
    let re = Regex::new(r"\[(?<text>[^\]]*)\]\((?<url>[^)]*)\)")?;
    let mut results = vec![];
    for captures in re.captures_iter(&content) {
        let text_match = captures.get(1).unwrap(); //.ok_or(BoredError::RegexError)?;
        let url_match = captures.get(2).unwrap(); //.ok_or(BoredError::RegexError)?;
        if let Ok(hyperlink) = Hyperlink::create(
            &captures["text"],
            (text_match.start(), text_match.end()),
            &captures["url"],
            (url_match.start(), url_match.end()),
        ) {
            results.push(hyperlink);
        }
    }
    Ok(results)
}

/// Returns text with URL and markdown charcters removed, plus a vetor of slices representing
/// the hyperlink locations
pub fn get_display(content: &str, hyperlinks: Vec<Hyperlink>) -> Display {
    let mut display = Display::new();
    let mut display_text = content.to_string();
    // goes backwards as if you remove the earliest first then later locations will be invalid
    for hyperlink in hyperlinks.iter().rev() {
        // remove link inclduing surrounding parenthesis
        let head = &display_text[0..hyperlink.link_location.0 - 1];
        let tail = &display_text[hyperlink.link_location.1 + 1..display_text.len()];
        let previous_len = display_text.len();
        display_text = head.to_owned() + tail;
        display.decrement_hyperlink_locations(previous_len - display_text.len());
        // remove markdown square brackets surrounding text
        let head = &display_text[0..hyperlink.text_location.0 - 1];
        let tail = &display_text[hyperlink.text_location.1 + 1..display_text.len()];
        let previous_len = display_text.len();
        display_text = head.to_owned() + &hyperlink.text + tail;
        display.decrement_hyperlink_locations(previous_len - display_text.len());
        // Only remove 1 from current hyperlink as only opening bracket [ affects the location
        display
            .hyperlink_locations
            .push((hyperlink.text_location.0 - 1, hyperlink.text_location.1 - 1));
    }
    display.hyperlink_locations.reverse();
    display.display_text = display_text;
    display
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_notice_relocate() {
        let bored = Bored::create("", Coordinate { x: 120, y: 40 });
        let mut notice = Notice::new();
        assert_eq!(notice.relocate(&bored, Coordinate { x: 10, y: 7 }), Ok(()));
        assert_eq!(notice.top_left, Coordinate { x: 10, y: 7 });
        assert_eq!(
            notice.relocate(&bored, Coordinate { x: 999, y: 999 }),
            Err(BoredError::NoticeOutOfBounds)
        );
    }

    #[test]
    fn test_get_max_chars() {
        let mut notice = Notice::new();
        notice.dimensions = Coordinate { x: 0, y: 0 };
        assert_eq!(notice.get_max_chars(), 0);
        notice.dimensions = Coordinate { x: 1, y: 0 };
        assert_eq!(notice.get_max_chars(), 0);
        notice.dimensions = Coordinate { x: 0, y: 1 };
        assert_eq!(notice.get_max_chars(), 0);
        notice.dimensions = Coordinate { x: 1, y: 1 };
        assert_eq!(notice.get_max_chars(), 0);
        notice.dimensions = Coordinate { x: 2, y: 2 };
        assert_eq!(notice.get_max_chars(), 0);
        notice.dimensions = Coordinate { x: 3, y: 3 };
        assert_eq!(notice.get_max_chars(), 1);
        notice.dimensions = Coordinate { x: 6, y: 9 };
        assert_eq!(notice.get_max_chars(), 28);
    }

    #[test]
    fn test_get_max_lines() {
        let mut notice = Notice::new();
        notice.dimensions = Coordinate { x: 0, y: 0 };
        assert_eq!(notice.get_max_lines(), 0);
        notice.dimensions = Coordinate { x: 2, y: 2 };
        assert_eq!(notice.get_max_lines(), 0);
        notice.dimensions = Coordinate { x: 3, y: 3 };
        assert_eq!(notice.get_max_lines(), 1);
    }

    #[test]
    fn test_get_display() -> Result<(), BoredError> {
        let content = "I am [BORED](Not)";
        let display_text = get_display(content, get_hyperlinks(content)?).display_text;
        assert_eq!(display_text, "I am BORED");
        let content = "I am [BORED](Not) at all ()[]() []";
        let display_text = get_display(content, get_hyperlinks(content)?).display_text;
        assert_eq!(display_text, "I am BORED at all () []");
        Ok(())
    }

    #[test]
    fn test_write() {
        let mut notice = Notice::new();
        notice.dimensions = Coordinate { x: 0, y: 0 };
        assert_eq!(notice.write("I am BORED"), Err(BoredError::TooMuchText));
        notice.dimensions = Coordinate { x: 7, y: 4 };
        assert_eq!(notice.write("I am BORED!"), Err(BoredError::TooMuchText));
        notice.dimensions = Coordinate { x: 7, y: 4 };
        assert_eq!(notice.write("I am BORED"), Ok(()));
        assert_eq!(notice.content, "I am BORED");
        notice.dimensions = Coordinate { x: 7, y: 4 };
        assert_eq!(notice.write("I\nam\nBORED"), Err(BoredError::TooMuchText));
        notice.dimensions = Coordinate { x: 7, y: 6 };
        assert_eq!(notice.write("I\nam\nBORED"), Ok(()));
        assert_eq!(notice.content, "I\nam\nBORED");
        notice.dimensions = Coordinate { x: 7, y: 4 };
        assert_eq!(
            notice.write("I am [BORED](NOT)!"),
            Err(BoredError::TooMuchText)
        );
        assert_eq!(notice.write("I am [BORED](NOT)"), Ok(()));
        assert_eq!(notice.content, "I am [BORED](NOT)");
    }

    #[test]
    fn test_decrement_hyperlink_locations() {
        let mut display = Display::new();
        display.decrement_hyperlink_locations(0);
        display.hyperlink_locations.push((0, 8));
        display.hyperlink_locations.push((20, 57));
        display.decrement_hyperlink_locations(1);
        let mut hyperlink_locations = vec![];
        hyperlink_locations.push((0, 7));
        hyperlink_locations.push((19, 56));
        assert_eq!(display.hyperlink_locations, hyperlink_locations);
    }

    #[test]
    fn test_hyperlinks_and_display() -> Result<(), BoredError> {
        let mut notice = Notice::new();
        let mut hyperlinks = get_hyperlinks(notice.get_content()).unwrap();
        assert!(hyperlinks.is_empty());
        notice.write("The [autonomi](https://autonomi.com/) website")?;
        hyperlinks = get_hyperlinks(&notice.content).unwrap();
        let mut links = vec![];
        let link =
            Hyperlink::create("autonomi", (5, 13), "https://autonomi.com/", (15, 36)).unwrap();
        links.push(link);
        assert_eq!(hyperlinks, links);
        let display_text = "The autonomi website";
        let display = get_display(&notice.get_content(), hyperlinks);
        assert_eq!(display.display_text, display_text);
        let mut test_display = Display::new();
        test_display.hyperlink_locations.push((4, 12));
        assert_eq!(
            display.hyperlink_locations,
            test_display.hyperlink_locations
        );
        let bored_address = BoredAddress::new();
        let content = format!(
            "{}, a [] () bored url: [bored]({})",
            notice.get_content(),
            bored_address
        );
        notice.write(&content)?;
        hyperlinks = get_hyperlinks(notice.get_content()).unwrap();
        let url_text = format!("{}", bored_address);
        let link = Hyperlink::create("bored", (67, 72), &url_text, (74, 74 + 8 + 64)).unwrap();
        links.push(link);
        assert_eq!(hyperlinks, links);
        let display_text = "The autonomi website, a [] () bored url: bored";
        let display = get_display(&notice.get_content(), hyperlinks);
        assert_eq!(display.display_text, display_text);
        let mut test_display = Display::new();
        test_display.hyperlink_locations.push((4, 12));
        test_display.hyperlink_locations.push((41, 46));
        assert_eq!(
            display.hyperlink_locations,
            test_display.hyperlink_locations
        );
        Ok(())
    }
}

use crate::{Bored, BoredAddress, BoredError, Coordinate};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt::{self};

/// Limit to avoid massive amount of text being accidentally put into hyperlink and making
/// bored to big to fit in scratchpadlonges
pub const MAX_URL_LENGTH: usize = 2048;

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

    pub fn get_link(&self) -> String {
        self.link.clone()
    }
}

/// a 2d vector of option<uszie> representing the location of hyperlinks
/// if the coordinate is empty it will be none otherwise it will be the
/// index of the hyperlink in Display.hyperlink_locations
#[derive(Debug, Clone, Default)]
pub struct NoticeHyperlinkMap {
    visible: Vec<Vec<Option<usize>>>,
}
impl Iterator for NoticeHyperlinkMap {
    type Item = Vec<Option<usize>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.visible.iter().next().cloned()
    }
}
impl fmt::Display for NoticeHyperlinkMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut display = String::new();
        for row in &self.visible {
            for coordinate in row {
                let text = match coordinate {
                    None => "*",
                    Some(notice_index) => &notice_index.to_string(),
                };
                display.push_str(text);
            }
            display.push_str("\n");
        }
        write!(f, "{}", display)
    }
}

impl NoticeHyperlinkMap {
    pub fn create(notice: &Notice) -> Result<NoticeHyperlinkMap, BoredError> {
        let content = notice.get_content();
        let display = get_display(content, get_hyperlinks(content)?);
        let mut visible =
            vec![vec![None; notice.dimensions.x as usize - 2]; notice.dimensions.y as usize - 2];
        let (mut x, mut y) = (0, 0);
        for (char_index, char) in display.display_text.chars().enumerate() {
            for (hyperlink_index, hyperlink_location) in
                display.hyperlink_locations.iter().enumerate()
            {
                for i in hyperlink_location.0..hyperlink_location.1 {
                    if char_index == i && char != '\n' {
                        visible[y][x] = Some(hyperlink_index);
                    }
                }
            }
            if char == '\n' {
                y += 1;
                x = 0;
            } else if x < notice.get_text_width() as usize - 1 {
                x += 1;
            } else {
                y += 1;
                x = 0;
            }
        }
        Ok(NoticeHyperlinkMap { visible })
    }

    pub fn get_map(&self) -> Vec<Vec<Option<usize>>> {
        self.visible.clone()
    }
}

/// Display contains the text to display plus a collections of the hyperlinks locations from left
/// to right
#[derive(Default, Debug)]
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

    pub fn get_hyperlink_locations(&self) -> Vec<(usize, usize)> {
        self.hyperlink_locations.clone()
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

    /// Width of visible text, ie width of notice minus two for the borders
    pub fn get_text_width(&self) -> u16 {
        if self.dimensions.x < 3 {
            0
        } else {
            self.dimensions.x - 2
        }
    }

    /// Height of visible text, ie width of notice minus two for the borders
    pub fn get_text_height(&self) -> u16 {
        if self.dimensions.y < 3 {
            0
        } else {
            self.dimensions.y - 2
        }
    }

    pub fn get_content(&self) -> &str {
        &self.content
    }

    pub fn get_display(&self) -> Result<Display, BoredError> {
        Ok(get_display(
            self.get_content(),
            get_hyperlinks(self.get_content())?,
        ))
    }

    /// moves notices position on board, both prior to placing and is called by Bored.add()
    pub fn relocate(&mut self, bored: &Bored, new_top_left: Coordinate) -> Result<(), BoredError> {
        let new_bottom_right = new_top_left.add(&self.dimensions);
        if new_bottom_right.within(&bored.dimensions) {
            self.top_left = new_top_left;
            return Ok(());
        }
        Err(BoredError::NoticeOutOfBounds(
            bored.dimensions,
            new_bottom_right,
        ))
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
        let display_lines = display_text.lines().count();
        let last_line = display_text.lines().last().unwrap_or_default();
        let used_chars = if display_lines > 0 {
            display_lines - 1
        } else {
            0
        } * self.get_text_width() as usize
            + last_line.chars().count();
        if used_chars > self.get_max_chars()
            || display_lines > self.get_max_lines()
            || (display_lines == self.get_max_lines()
                && last_line.chars().last().unwrap_or_default() == '\n')
            || (display_lines == self.get_max_lines()
                && last_line.chars().count() > self.get_text_width() as usize)
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
        let text_match = captures.get(1).ok_or(BoredError::RegexError)?;
        let url_match = captures.get(2).ok_or(BoredError::RegexError)?;
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
            Err(BoredError::NoticeOutOfBounds(
                bored.get_dimensions(),
                Coordinate { x: 1059, y: 1017 }
            ))
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
        notice.dimensions = Coordinate { x: 12, y: 3 };
        assert_eq!(notice.write("I am BORED!"), Err(BoredError::TooMuchText));
        notice.dimensions = Coordinate { x: 12, y: 3 };
        assert_eq!(notice.write("I am BORED"), Ok(()));
        assert_eq!(notice.content, "I am BORED");
        notice.dimensions = Coordinate { x: 12, y: 4 };
        assert_eq!(notice.write("I\nam\nBORED"), Err(BoredError::TooMuchText));
        notice.dimensions = Coordinate { x: 12, y: 5 };
        assert_eq!(notice.write("I\nam\nBORED"), Ok(()));
        assert_eq!(notice.content, "I\nam\nBORED");
        notice.dimensions = Coordinate { x: 12, y: 3 };
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
        // Test links split over lines
        notice.write("The [auto\nnomi](https://autonomi.com/) website")?;
        hyperlinks = get_hyperlinks(&notice.content).unwrap();
        let mut links = vec![];
        let link =
            Hyperlink::create("auto\nnomi", (5, 14), "https://autonomi.com/", (16, 37)).unwrap();
        links.push(link);
        assert_eq!(hyperlinks, links);
        let display_text = "The auto\nnomi website";
        let display = get_display(&notice.get_content(), hyperlinks);
        assert_eq!(display.display_text, display_text);
        Ok(())
    }

    #[test]
    fn test_notice_hyperlink_map() -> Result<(), BoredError> {
        let mut notice = Notice::create(Coordinate { x: 10, y: 13 });
        notice.write(
            "We are [link](url) [bored](url).\nYou are [link](url) bored.\nI am [boooo\nooored](url).\nHello\nWorld",
        )?;
        let notice_hyperlink_map = NoticeHyperlinkMap::create(&notice)?;
        let expected_output = r#"*******0
000*1111
1*******
********
2222****
********
*****333
33******
333333**
********
********
"#;
        assert_eq!(expected_output, format!("{}", notice_hyperlink_map));
        eprintln!("{}", notice_hyperlink_map);
        Ok(())
    }
}

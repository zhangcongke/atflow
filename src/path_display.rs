use std::path::Path;

pub fn clip_middle(input: &str, max_width: usize) -> String {
    crate::path_display::implementation::clip_middle(input, max_width)
}

pub fn display_path(path: &Path, expanded: bool, max_width: usize) -> String {
    crate::path_display::implementation::display_path(path, expanded, max_width)
}

mod implementation {
    use std::path::Path;

    pub fn clip_middle(input: &str, max_width: usize) -> String {
        if input.chars().count() <= max_width {
            return input.to_owned();
        }
        if max_width <= 1 {
            return ".".to_owned();
        }
        if max_width <= 3 {
            return ".".repeat(max_width);
        }
        let ellipsis = "...";
        let available = max_width - ellipsis.len();
        let left = available / 2;
        let right = available - left;
        let prefix: String = input.chars().take(left).collect();
        let suffix: String = input
            .chars()
            .rev()
            .take(right)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("{prefix}{ellipsis}{suffix}")
    }

    pub fn display_path(path: &Path, expanded: bool, max_width: usize) -> String {
        let text = path.display().to_string();
        if expanded {
            text
        } else {
            clip_middle(&text, max_width)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn leaves_short_text_unchanged() {
        assert_eq!(clip_middle("src/main.rs", 30), "src/main.rs");
    }

    #[test]
    fn clips_long_text_in_the_middle() {
        assert_eq!(
            clip_middle("/home/congke/work/ntl-imputation/data/nightlight", 28),
            "/home/congke...ta/nightlight"
        );
    }

    #[test]
    fn expanded_display_returns_full_path() {
        let path = PathBuf::from("/home/congke/work/ntl-imputation/data/nightlight");
        assert_eq!(
            display_path(&path, true, 20),
            "/home/congke/work/ntl-imputation/data/nightlight"
        );
    }

    #[test]
    fn collapsed_display_uses_width_limit() {
        let path = PathBuf::from("/home/congke/work/ntl-imputation/data/nightlight");
        assert_eq!(display_path(&path, false, 20), "/home/co...ightlight");
    }
}

//! Minimal parser for Valve's text KeyValues format (`.vdf` / `.acf`).

use anyhow::{Result, bail};

#[derive(Debug, Clone, PartialEq)]
pub enum Vdf {
    Str(String),
    Map(Vec<(String, Vdf)>),
}

impl Vdf {
    /// Case-insensitive child lookup (Valve is not consistent about key casing).
    pub fn get(&self, key: &str) -> Option<&Vdf> {
        self.entries().iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v)
    }

    pub fn str(&self, key: &str) -> Option<&str> {
        match self.get(key)? {
            Vdf::Str(s) => Some(s),
            Vdf::Map(_) => None,
        }
    }

    pub fn entries(&self) -> &[(String, Vdf)] {
        match self {
            Vdf::Map(entries) => entries,
            Vdf::Str(_) => &[],
        }
    }
}

#[derive(Debug, PartialEq)]
enum Token {
    Str(String),
    Open,
    Close,
}

fn tokenize(text: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            c if c.is_whitespace() || c == '\u{feff}' => {
                chars.next();
            }
            '{' => {
                chars.next();
                tokens.push(Token::Open);
            }
            '}' => {
                chars.next();
                tokens.push(Token::Close);
            }
            '/' => {
                // `//` line comment
                chars.next();
                if chars.peek() == Some(&'/') {
                    for c in chars.by_ref() {
                        if c == '\n' {
                            break;
                        }
                    }
                } else {
                    bail!("unexpected '/' in vdf");
                }
            }
            '[' => {
                // Platform conditionals such as [$WIN32] are ignored.
                for c in chars.by_ref() {
                    if c == ']' {
                        break;
                    }
                }
            }
            '"' => {
                chars.next();
                let mut s = String::new();
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some(other) => s.push(other),
                            None => bail!("unterminated escape in vdf"),
                        },
                        Some(other) => s.push(other),
                        None => bail!("unterminated string in vdf"),
                    }
                }
                tokens.push(Token::Str(s));
            }
            _ => {
                let mut s = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_whitespace() || c == '{' || c == '}' || c == '"' {
                        break;
                    }
                    s.push(c);
                    chars.next();
                }
                tokens.push(Token::Str(s));
            }
        }
    }
    Ok(tokens)
}

/// Parses a KeyValues document. The result is a map of the top-level pairs.
pub fn parse(text: &str) -> Result<Vdf> {
    let tokens = tokenize(text)?;
    let mut iter = tokens.into_iter();
    parse_map(&mut iter, false)
}

fn parse_map(tokens: &mut impl Iterator<Item = Token>, nested: bool) -> Result<Vdf> {
    let mut entries = Vec::new();
    loop {
        match tokens.next() {
            Some(Token::Str(key)) => match tokens.next() {
                Some(Token::Str(value)) => entries.push((key, Vdf::Str(value))),
                Some(Token::Open) => entries.push((key, parse_map(tokens, true)?)),
                _ => bail!("expected value after key '{key}'"),
            },
            Some(Token::Close) if nested => return Ok(Vdf::Map(entries)),
            Some(Token::Close) => bail!("unbalanced '}}' in vdf"),
            Some(Token::Open) => bail!("unexpected '{{' in vdf"),
            None if nested => bail!("unexpected end of vdf"),
            None => return Ok(Vdf::Map(entries)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_library_folders() {
        let text = r#"
"libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
		"apps"
		{
			"228980"		"314399871"
		}
	}
	// comment
	"1" { "path" "D:\\SteamLibrary" }
}"#;
        let doc = parse(text).unwrap();
        let folders = doc.get("LibraryFolders").unwrap();
        assert_eq!(folders.entries().len(), 2);
        assert_eq!(folders.get("0").unwrap().str("path"), Some(r"C:\Program Files (x86)\Steam"));
        assert_eq!(folders.get("1").unwrap().str("path"), Some(r"D:\SteamLibrary"));
        assert_eq!(folders.get("0").unwrap().get("apps").unwrap().str("228980"), Some("314399871"));
    }

    #[test]
    fn rejects_unbalanced() {
        assert!(parse(r#""a" { "b" "c" "#).is_err());
    }
}

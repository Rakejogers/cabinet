//! Themes are plain CSS files that set custom properties on `:root`. Two are
//! compiled in; any `*.css` file in the user's themes folder is added to the
//! list at runtime.

use std::fs;
use std::path::Path;

use serde::Serialize;

const BUILTIN: &[(&str, &str)] =
    &[("dark", include_str!("../themes/dark.css")), ("light", include_str!("../themes/light.css"))];

const README: &str = "Cabinet themes\n\
==============\n\n\
Every *.css file in this folder appears in Cabinet's theme switcher.\n\
Start from one of the *.css.example files: copy it, rename it to end in .css,\n\
and change the values. The first line comment `/* name: My Theme */` sets the\n\
name shown in the switcher. Press the rescan button in Cabinet to reload themes.\n";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    pub id: String,
    pub name: String,
    pub css: String,
    pub builtin: bool,
}

/// Reads `/* name: Something */` from the top of a theme file.
fn display_name(css: &str, fallback: &str) -> String {
    css.lines()
        .take(5)
        .find_map(|line| {
            let rest = line.trim().strip_prefix("/*")?.trim_start();
            let rest = rest.strip_prefix("name:")?;
            Some(rest.trim_end_matches("*/").trim().to_string())
        })
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

/// Creates the user themes folder with a README and copies of the built-ins.
pub fn init_dir(dir: &Path) {
    if fs::create_dir_all(dir).is_err() {
        return;
    }
    let readme = dir.join("README.txt");
    if !readme.exists() {
        let _ = fs::write(&readme, README);
        for (id, css) in BUILTIN {
            let _ = fs::write(dir.join(format!("{id}.css.example")), css);
        }
    }
}

pub fn list(dir: &Path) -> Vec<Theme> {
    let mut themes: Vec<Theme> = BUILTIN
        .iter()
        .map(|(id, css)| Theme {
            id: (*id).to_string(),
            name: display_name(css, id),
            css: (*css).to_string(),
            builtin: true,
        })
        .collect();

    let mut user: Vec<Theme> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "css"))
        .filter_map(|e| {
            let path = e.path();
            let stem = path.file_stem()?.to_string_lossy().to_string();
            let css = fs::read_to_string(&path).ok()?;
            Some(Theme { id: format!("user:{stem}"), name: display_name(&css, &stem), css, builtin: false })
        })
        .collect();
    user.sort_by_key(|t| t.name.to_lowercase());
    themes.extend(user);
    themes
}

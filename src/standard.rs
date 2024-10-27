use std::collections::HashMap;

/// Load all standards. Tone tag standards can be found in `/standards/*`.
///
/// By default, users have only the `core` standard enabled.
pub fn load_all_stds() -> Result<Vec<(&'static str, Standard<'static>)>, &'static str> {
    Ok(vec![
        ("core", load_std(include_str!("../standards/core.md"))?),
        (
            "common-namtao",
            load_std(include_str!("../standards/common_namtao.md"))?,
        ),
    ])
}

#[derive(Debug)]
pub struct Standard<'a> {
    pub title: &'a str,
    pub description: String,
    pub tags: HashMap<&'a str, String>,
}

pub fn load_std(str: &str) -> Result<Standard, &'static str> {
    enum Head<'a> {
        Title(&'a str),
        Tags(Vec<&'a str>),
    }

    let mut cont = None;
    let mut head = None;

    let mut title = None;
    let mut desc = None;
    let mut tags = HashMap::new();

    for line in str.lines() {
        if line.starts_with("###") {
            return Err("No rule defined for '###'");
        } else if let Some(line) = line.strip_prefix("##") {
            match head {
                Some(Head::Title(s)) => {
                    if title.replace(s).is_some() {
                        return Err("Title is already defined");
                    }
                    if desc.replace(cont.take().unwrap_or_default()).is_some() {
                        return Err("Title is already defined");
                    }
                }
                Some(Head::Tags(t)) => {
                    for t in t {
                        tags.insert(t, cont.clone().unwrap_or_default());
                    }
                    cont.take();
                }
                None => (),
            }

            head = Some(Head::Tags(
                line.split(' ').filter(|x| !x.trim().is_empty()).collect(),
            ));
            cont = Some(String::new());
        } else if let Some(line) = line.strip_prefix("#") {
            match head {
                Some(Head::Title(s)) => {
                    if title.replace(s).is_some() {
                        return Err("Title is already defined");
                    }
                    if desc.replace(cont.take().unwrap_or_default()).is_some() {
                        return Err("Title is already defined");
                    }
                }
                Some(Head::Tags(t)) => {
                    for t in t {
                        tags.insert(t, cont.clone().unwrap_or_default());
                    }
                    cont.take();
                }
                None => (),
            }

            head = Some(Head::Title(line.trim()));
            cont = Some(String::new());
        } else {
            if cont.is_none() {
                return Err("Filling a missing tag");
            }
            cont.as_mut().unwrap().push_str(line.trim());
            cont.as_mut().unwrap().push('\n');
        }
    }

    match head {
        Some(Head::Title(s)) => {
            assert!(title.replace(s).is_none());
            assert!(desc.replace(cont.replace(String::new()).unwrap()).is_none());
        }
        Some(Head::Tags(t)) => {
            for t in t {
                tags.insert(t, cont.clone().unwrap());
            }
            cont.replace(String::new());
        }
        None => (),
    }

    Ok(Standard {
        title: match title {
            Some(x) => x,
            None => return Err("No title"),
        },
        description: desc.unwrap_or_default(),
        tags,
    })
}

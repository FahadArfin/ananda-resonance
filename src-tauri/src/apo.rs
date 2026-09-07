use crate::model::{validate_profile, Filter, Point, Profile};
use regex::Regex;
use std::sync::LazyLock;

static FILTER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^Filter\s+\d+:\s+(ON|OFF)\s+(PK|LSC|HSC|LS|HS|HPQ|LPQ|HP|LP)\s+Fc\s+([\d.eE+\-]+)\s+Hz(?:\s+Gain\s+([\d.eE+\-]+)\s+dB)?(?:\s+Q\s+([\d.eE+\-]+))?\s*$").unwrap()
});
static PREAMP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^Preamp:\s*([\d.eE+\-]+)\s+dB$").unwrap());

pub fn parse(text: &str, name: &str, provenance: &str) -> Result<Profile, String> {
    if text.len() > 1_048_576 {
        return Err("Preset exceeds 1 MB".into());
    }
    let mut p = Profile {
        schema_version: 1,
        id: uuid::Uuid::new_v4().to_string(),
        name: name.into(),
        icon: "headphones".into(),
        headphone_id: "default".into(),
        preamp: 0.0,
        filters: vec![],
        graphic_eq: vec![],
        provenance: provenance.into(),
    };
    let mut seen_preamp = false;
    let mut recognized = false;
    for (i, line) in text.trim_start_matches('\u{feff}').lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let err = |message: &str| format!("Line {}: {}", i + 1, message);
        if let Some(c) = PREAMP.captures(line) {
            if seen_preamp {
                return Err(err("Multiple preamps are not supported"));
            }
            p.preamp = c[1].parse().map_err(|_| err("Invalid preamp"))?;
            seen_preamp = true;
            recognized = true;
        } else if let Some(c) = FILTER.captures(line) {
            let raw_kind = c[2].to_uppercase();
            let kind = match raw_kind.as_str() {
                "LS" => "LSC",
                "HS" => "HSC",
                "HP" => "HPQ",
                "LP" => "LPQ",
                k => k,
            };
            let pass = kind == "HPQ" || kind == "LPQ";
            if !pass && c.get(4).is_none() {
                return Err(err("This filter requires Gain"));
            }
            if pass && c.get(4).is_some() {
                return Err(err("Pass filters do not support Gain"));
            }
            if ["PK", "LSC", "HSC", "HPQ", "LPQ"].contains(&raw_kind.as_str()) && c.get(5).is_none()
            {
                return Err(err("This filter requires Q"));
            }
            p.filters.push(Filter {
                id: uuid::Uuid::new_v4().to_string(),
                kind: kind.into(),
                enabled: c[1].eq_ignore_ascii_case("ON"),
                frequency: c[3].parse().map_err(|_| err("Invalid frequency"))?,
                gain: c
                    .get(4)
                    .map(|v| v.as_str().parse())
                    .transpose()
                    .map_err(|_| err("Invalid gain"))?
                    .unwrap_or(0.0),
                q: c.get(5)
                    .map(|v| v.as_str().parse())
                    .transpose()
                    .map_err(|_| err("Invalid Q"))?
                    .unwrap_or(std::f64::consts::FRAC_1_SQRT_2),
            });
            recognized = true;
        } else if line.to_lowercase().starts_with("graphiceq:") {
            if !p.graphic_eq.is_empty() {
                return Err(err("Multiple GraphicEQ directives are not supported"));
            }
            for item in line.split_once(':').unwrap().1.split(';') {
                let values: Vec<_> = item.split_whitespace().collect();
                if values.len() != 2 {
                    return Err(err("Each GraphicEQ point needs frequency and gain"));
                }
                p.graphic_eq.push(Point {
                    frequency: values[0].parse().map_err(|_| err("Invalid frequency"))?,
                    gain: values[1].parse().map_err(|_| err("Invalid gain"))?,
                });
            }
            recognized = true;
        } else {
            return Err(err(&format!(
                "Unsupported directive: {line}. Import cancelled; no commands were executed."
            )));
        }
    }
    if !recognized {
        return Err("No supported EQ data found".into());
    }
    validate_profile(&p)?;
    Ok(p)
}

pub fn export(p: &Profile) -> Result<String, String> {
    validate_profile(p)?;
    let mut out = format!("# {}\nPreamp: {} dB\n", p.name, p.preamp);
    for (i, f) in p.filters.iter().enumerate() {
        let gain = if f.kind == "HPQ" || f.kind == "LPQ" {
            String::new()
        } else {
            format!(" Gain {} dB", f.gain)
        };
        out.push_str(&format!(
            "Filter {}: {} {} Fc {} Hz{} Q {}\n",
            i + 1,
            if f.enabled { "ON" } else { "OFF" },
            f.kind,
            f.frequency,
            gain,
            f.q
        ));
    }
    if !p.graphic_eq.is_empty() {
        out.push_str(&format!(
            "GraphicEQ: {}\n",
            p.graphic_eq
                .iter()
                .map(|v| format!("{} {}", v.frequency, v.gain))
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }
    Ok(out)
}

pub fn generate(p: &Profile, device: &str) -> Result<String, String> {
    if !Regex::new(
        r"^\{[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\}$",
    )
    .unwrap()
    .is_match(device)
    {
        return Err("Select a valid Windows output endpoint".into());
    }
    let out = format!(
        "# Managed by Ananda Control. Changes are detected.\nDevice: {device}\nChannel: all\n{}",
        export(p)?
    );
    // Parse the generated DSP portion again before committing.
    parse(&export(p)?, &p.name, "validation")?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn supplied_values_and_midpoint() {
        let db = crate::model::defaults();
        assert_eq!(db.profiles.len(), 6);
        assert_eq!(db.profiles[0].preamp, -5.5);
        assert_eq!(
            db.profiles[0]
                .filters
                .iter()
                .map(|f| f.gain)
                .collect::<Vec<_>>(),
            vec![5.0, -1.0, 0.5, 5.5, -3.0, -2.0, -4.0]
        );
        assert_eq!(
            db.profiles[2]
                .filters
                .iter()
                .map(|f| f.gain)
                .collect::<Vec<_>>(),
            vec![6.0, -0.25, 0.5, 5.0, -2.75, -1.5, -3.25]
        );
        assert_eq!(db.profiles[2].preamp, -7.0);
    }
    #[test]
    fn rejects_commands_without_partial_import() {
        for text in [
            "Preamp: -5 dB\nInclude: peace.txt",
            "Device: all",
            "Filter 1: ON PK Fc NaN Hz Gain 2 dB Q 1",
            "Preamp: -5 dB\nPreamp: -2 dB",
            "",
            "Filter 1: ON PK Fc 100 Hz Gain 2 dB",
        ] {
            assert!(parse(text, "test", "").is_err(), "{text}");
        }
    }
    #[test]
    fn parametric_roundtrip() {
        for p in crate::model::defaults().profiles {
            let restored = parse(&export(&p).unwrap(), &p.name, "").unwrap();
            assert_eq!(restored.preamp, p.preamp);
            for (a, b) in restored.filters.iter().zip(&p.filters) {
                assert_eq!(
                    (a.frequency, a.gain, a.q, &a.kind, a.enabled),
                    (b.frequency, b.gain, b.q, &b.kind, b.enabled)
                );
            }
        }
    }
    #[test]
    fn graphic_roundtrip() {
        let p = parse("GraphicEQ: 20 -3; 100 0; 10000 2", "graphic", "").unwrap();
        assert_eq!(
            parse(&export(&p).unwrap(), "graphic", "")
                .unwrap()
                .graphic_eq,
            p.graphic_eq
        );
    }
    #[test]
    fn rejects_bad_graphic_order() {
        assert!(parse("GraphicEQ: 100 0; 20 -3", "g", "").is_err());
    }
    #[test]
    fn all_filter_types_roundtrip() {
        let p = parse("Filter 1: ON LSC Fc 100 Hz Gain 2 dB Q 0.7\nFilter 2: OFF HSC Fc 8000 Hz Gain -1 dB Q 0.8\nFilter 3: ON HPQ Fc 20 Hz Q 0.707\nFilter 4: ON LPQ Fc 18000 Hz Q 0.8", "types", "").unwrap();
        assert_eq!(
            parse(&export(&p).unwrap(), "types", "")
                .unwrap()
                .filters
                .len(),
            4
        );
    }
    #[test]
    fn stock_and_device_scope() {
        let db = crate::model::defaults();
        let text = generate(
            db.profiles.last().unwrap(),
            "{63f8b8a3-7684-462c-b6ab-5a68228fab30}",
        )
        .unwrap();
        assert!(text.contains("Preamp: 0 dB"));
        assert!(!text.contains("Filter"));
        assert!(generate(&db.profiles[0], "all\nInclude: evil").is_err());
    }
    #[test]
    fn bounds_and_nonfinite() {
        let mut p = crate::model::defaults().profiles.remove(0);
        p.filters[0].q = 0.0;
        assert!(validate_profile(&p).is_err());
        p.filters[0].q = 1.0;
        p.preamp = f64::NAN;
        assert!(validate_profile(&p).is_err());
    }
}

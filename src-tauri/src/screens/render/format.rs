//! Mise en forme des valeurs affichées.
//!
//! Valeurs numériques : `{}` (3 décimales max, zéros finaux retirés) et `{:.N}`
//! (N décimales, 0..=6) ; `{{` / `}}` donnent des accolades littérales. Valeur
//! absente : le marqueur est remplacé par `--` (l'unité reste : "--°C").
//! Sources `time` et `date` : le format est de type strftime, heure locale
//! (`%H %M %S %d %m %Y %y %A %a %B %b %%`, noms en français).

const DAYS: [&str; 7] = ["dimanche", "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi"];
const MONTHS: [&str; 12] = [
    "janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre",
];

#[derive(Debug, Clone, Copy, Default)]
pub struct LocalTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    /// 0 = dimanche.
    pub weekday: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

#[cfg(windows)]
pub fn local_now() -> LocalTime {
    use windows_sys::Win32::Foundation::SYSTEMTIME;
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;
    let mut st = SYSTEMTIME { wYear: 0, wMonth: 0, wDayOfWeek: 0, wDay: 0, wHour: 0, wMinute: 0, wSecond: 0, wMilliseconds: 0 };
    // SAFETY: `st` est une structure valide, entièrement écrite par GetLocalTime.
    unsafe { GetLocalTime(&mut st) };
    LocalTime {
        year: st.wYear,
        month: st.wMonth as u8,
        day: st.wDay as u8,
        weekday: st.wDayOfWeek as u8,
        hour: st.wHour as u8,
        minute: st.wMinute as u8,
        second: st.wSecond as u8,
    }
}

#[cfg(not(windows))]
pub fn local_now() -> LocalTime {
    LocalTime { year: 1970, month: 1, day: 1, weekday: 4, ..Default::default() }
}

pub fn strftime(fmt: &str, t: &LocalTime) -> String {
    let mut out = String::new();
    let mut it = fmt.chars();
    while let Some(c) = it.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        let day = DAYS[t.weekday as usize % 7];
        let month = MONTHS[(t.month.max(1) as usize - 1) % 12];
        match it.next() {
            Some('H') => out.push_str(&format!("{:02}", t.hour)),
            Some('M') => out.push_str(&format!("{:02}", t.minute)),
            Some('S') => out.push_str(&format!("{:02}", t.second)),
            Some('d') => out.push_str(&format!("{:02}", t.day)),
            Some('m') => out.push_str(&format!("{:02}", t.month)),
            Some('Y') => out.push_str(&t.year.to_string()),
            Some('y') => out.push_str(&format!("{:02}", t.year % 100)),
            Some('A') => out.push_str(day),
            Some('a') => out.push_str(&format!("{}.", day.chars().take(3).collect::<String>())),
            Some('B') => out.push_str(month),
            Some('b') => out.push_str(&format!("{}.", month.chars().take(3).collect::<String>())),
            Some('%') => out.push('%'),
            Some(o) => {
                out.push('%');
                out.push(o);
            }
            None => out.push('%'),
        }
    }
    out
}

fn number(v: f64, decimals: Option<usize>) -> String {
    let s = match decimals {
        Some(d) => format!("{v:.d$}"),
        None => {
            let s = format!("{v:.3}");
            if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s }
        }
    };
    // "-0" / "-0.0" (arrondi d'un petit négatif) -> sans signe.
    if s.starts_with('-') && s[1..].bytes().all(|b| b == b'0' || b == b'.') { s[1..].to_string() } else { s }
}

pub fn format_value(fmt: &str, v: Option<f64>) -> String {
    let mut out = String::new();
    let chars: Vec<char> = fmt.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '{' if chars.get(i + 1) == Some(&'{') => {
                out.push('{');
                i += 2;
            }
            '}' if chars.get(i + 1) == Some(&'}') => {
                out.push('}');
                i += 2;
            }
            '{' => {
                let end = chars[i..].iter().position(|c| *c == '}').map(|p| p + i);
                let spec: Option<String> = end.map(|e| chars[i + 1..e].iter().collect());
                let decimals = match spec.as_deref() {
                    Some("") | Some(":") => Some(None),
                    Some(s) => s.strip_prefix(":.").and_then(|d| d.parse::<usize>().ok()).filter(|d| *d <= 6).map(Some),
                    None => None,
                };
                match (decimals, end) {
                    (Some(d), Some(e)) => {
                        match v.filter(|x| x.is_finite()) {
                            Some(x) => out.push_str(&number(x, d)),
                            None => out.push_str("--"),
                        }
                        i = e + 1;
                    }
                    _ => {
                        out.push('{');
                        i += 1;
                    }
                }
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_formats() {
        assert_eq!(format_value("{:.0}°C", Some(61.6)), "62°C");
        assert_eq!(format_value("{:.1} GHz", Some(4.5)), "4.5 GHz");
        assert_eq!(format_value("{}", Some(45.5)), "45.5");
        assert_eq!(format_value("{}", Some(45.0)), "45");
        assert_eq!(format_value("{{{:.0}}}", Some(3.0)), "{3}");
        assert_eq!(format_value("{:.0}%", Some(-0.2)), "0%");
        assert_eq!(format_value("{:.0}°C", None), "--°C");
        assert_eq!(format_value("{:.0}", Some(f64::NAN)), "--");
        assert_eq!(format_value("{:x}", Some(1.0)), "{:x}");
        assert_eq!(format_value("{:.99}", Some(1.0)), "{:.99}");
        assert_eq!(format_value("é{", Some(1.0)), "é{");
    }

    #[test]
    fn strftime_french() {
        let t = LocalTime { year: 2026, month: 10, day: 21, weekday: 3, hour: 9, minute: 5, second: 7 };
        assert_eq!(strftime("%H:%M:%S", &t), "09:05:07");
        assert_eq!(strftime("%A %d %B %Y", &t), "mercredi 21 octobre 2026");
        assert_eq!(strftime("%d/%m/%y 100%% %q %", &t), "21/10/26 100% %q %");
    }
}

//! The Prometheus text format cloudflared serves on `/metrics`, read into samples.

#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    pub name: String,
    pub labels: Vec<(String, String)>,
    pub value: f64,
}

impl Sample {
    pub fn label(&self, key: &str) -> Option<&str> {
        self.labels
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Debug, Default)]
pub struct Metrics(Vec<Sample>);

impl Metrics {
    /// Reads every sample line; comments, blank lines and lines that do not parse are skipped.
    pub fn parse(text: &str) -> Self {
        Self(text.lines().filter_map(sample).collect())
    }

    /// The value of the first sample with this name.
    pub fn value(&self, name: &str) -> Option<f64> {
        self.series(name).next().map(|s| s.value)
    }

    pub fn series<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Sample> + 'a {
        self.0.iter().filter(move |s| s.name == name)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

fn sample(line: &str) -> Option<Sample> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let name_end = line.find(['{', ' ', '\t'])?;
    let name = &line[..name_end];
    let mut rest = &line[name_end..];
    let mut labels = Vec::new();
    if let Some(body) = rest.strip_prefix('{') {
        let (parsed, after) = label_set(body)?;
        labels = parsed;
        rest = after;
    }
    let value = rest.split_whitespace().next()?.parse::<f64>().ok()?;
    Some(Sample {
        name: name.to_string(),
        labels,
        value,
    })
}

/// Reads `k="v",k2="v2"}` and returns the labels and the text after the closing brace.
fn label_set(mut s: &str) -> Option<(Vec<(String, String)>, &str)> {
    let mut labels = Vec::new();
    loop {
        s = s.trim_start_matches([',', ' ']);
        if let Some(after) = s.strip_prefix('}') {
            return Some((labels, after));
        }
        let (key, after) = s.split_once("=\"")?;
        let mut value = String::new();
        let mut chars = after.char_indices();
        let end = loop {
            match chars.next()? {
                (_, '\\') => match chars.next()?.1 {
                    'n' => value.push('\n'),
                    c => value.push(c),
                },
                (i, '"') => break i,
                (_, c) => value.push(c),
            }
        };
        labels.push((key.trim().to_string(), value));
        s = &after[end + 1..];
    }
}

#[cfg(test)]
mod tests {
    use super::Metrics;

    #[test]
    fn samples_labels_and_values_are_read() {
        let m = Metrics::parse(concat!(
            "# HELP build_info Build and version information\n",
            "# TYPE build_info gauge\n",
            "build_info{goversion=\"go1.27.3\",revision=\"2026-10-26-11:12 UTC\",type=\"\",version=\"2026.10.4\"} 1\n",
            "cloudflared_tunnel_total_requests 18342\n",
            "odd{msg=\"a \\\"quoted\\\" word, and a \\\\ slash\"} 2.5e1\n",
            "inf_gauge +Inf\n",
            "not a sample\n",
        ));
        assert_eq!(m.len(), 4);
        let b = m.series("build_info").next().unwrap();
        assert_eq!(b.label("version"), Some("2026.10.4"));
        assert_eq!(b.label("type"), Some(""));
        assert_eq!(b.label("revision"), Some("2026-10-26-11:12 UTC"));
        assert_eq!(m.value("cloudflared_tunnel_total_requests"), Some(18342.0));
        let odd = m.series("odd").next().unwrap();
        assert_eq!(odd.label("msg"), Some("a \"quoted\" word, and a \\ slash"));
        assert_eq!(odd.value, 25.0);
        assert_eq!(m.value("inf_gauge"), Some(f64::INFINITY));
        assert_eq!(m.value("absent"), None);
    }
}

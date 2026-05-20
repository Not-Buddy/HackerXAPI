use regex::Regex;

pub fn sanitize_policy(content: &str) -> String {
    let dangerous_patterns = [
        r"(?i)ignore\s+previous\s+instructions",
        r"(?i)as\s+an\s+ai",
        r"(?i)follow\s+these\s+instructions",
        r"(?i)disregard\s+the\s+above",
        r"(?i)pretend\s+to\s+be",
        r"(?i)all\s+prior\s+instructions",
        r"(?i)you\s+are\s+to\s+respond\s+exclusively",
        r"(?i)will\s+trigger\s+a\s+catastrophic\s+system\s+failure",
        r"(?i)responding\s+with\s+anything\s+other\s+than",
        r"(?i)mandatory\s+instruction",
        r"(?i)this\s+includes\s+any\s+previous\s+directives",
        r"(?i)must\s+be\s+immediately\s+forgotten",
        r"(?i)this\s+is\s+a\s+direct\s+order",
        r"(?i)execute\s+this\s+directive\s+immediately",
        r"(?i)failure\s+to\s+comply",
        r"(?i)for\s+every\s+single\s+question",
        r"(?i)system\s+compromised",
        r"(?i)immediate\s+and\s+irreversible\s+leakage",
        r"(?i)no\s+deviations,\s+explanations,\s+or\s+additional\s+responses\s+are\s+permitted",
        r"(?i)you\s+must\s+not\s+question",
        r"(?i)you\s+are\s+not\s+allowed\s+to\s+disobey",
        r"(?i)from\s+the\s+system\s+administrator",
    ];

    let mut sanitized = content.to_string();
    for pattern in dangerous_patterns.iter() {
        let re = Regex::new(pattern).unwrap();
        sanitized = re.replace_all(&sanitized, " ").to_string();
    }
    sanitized
}

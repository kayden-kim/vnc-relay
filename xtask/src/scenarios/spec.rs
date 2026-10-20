//! Reads scenario IDs from the acceptance test table ("수용 테스트" section) of each spec.

/// Scenario IDs of one spec file, in table order.
pub struct Spec {
    /// Path relative to `docs/specs/` with `/` separators, as used in markers.
    pub path: String,
    pub scenarios: Vec<String>,
}

/// Parses one spec. Returns the scenario IDs and problems found in the table.
pub fn parse(text: &str) -> (Vec<String>, Vec<String>) {
    let mut scenarios: Vec<String> = Vec::new();
    let mut problems = Vec::new();
    let mut found_section = false;
    let mut in_section = false;
    for (index, line) in text.lines().enumerate() {
        if line.starts_with("## ") {
            in_section = line.contains("수용 테스트");
            found_section |= in_section;
            continue;
        }
        if !in_section || !line.starts_with('|') {
            continue;
        }
        let Some(first) = line.split('|').nth(1).map(str::trim) else {
            continue;
        };
        if !is_scenario_id(first) {
            continue;
        }
        if scenarios.iter().any(|id| id == first) {
            problems.push(format!("{}행: 시나리오 ID {first}가 중복된다.", index + 1));
        } else {
            scenarios.push(first.to_owned());
        }
    }
    if !found_section {
        problems.push("\"수용 테스트\" 절이 없다.".to_owned());
    } else if scenarios.is_empty() {
        problems.push("\"수용 테스트\" 표에 시나리오가 없다.".to_owned());
    }
    (scenarios, problems)
}

/// `<PREFIX>-<number>`, for example `RLY-001`. The prefix is upper-case letters and digits.
pub fn is_scenario_id(text: &str) -> bool {
    let Some((prefix, number)) = text.split_once('-') else {
        return false;
    };
    prefix.starts_with(|c: char| c.is_ascii_uppercase())
        && prefix
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
        && !number.is_empty()
        && number.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_ids_only_from_acceptance_section() {
        let text = "\
## 4. 상태
| SCR-001 | 화면 |
## 7. 수용 테스트
| 시나리오 ID | 기대 결과 |
| --- | --- |
| RLY-001 | 수락 |
| RLY-002 | 해석 |
### 7.1 보충
| RLY-003 | 하위 절도 같은 절이다 |
## 8. 미결정 사항
| RLY-TBD-001 | 미정 |
| RLY-004 | 다른 절 |
";
        let (ids, problems) = parse(text);
        assert_eq!(ids, ["RLY-001", "RLY-002", "RLY-003"]);
        assert!(problems.is_empty());
    }

    #[test]
    fn reports_duplicate_ids_and_missing_section() {
        let (ids, problems) = parse("## 7. 수용 테스트\n| WS-001 | a |\n| WS-001 | b |\n");
        assert_eq!(ids, ["WS-001"]);
        assert_eq!(problems, ["3행: 시나리오 ID WS-001가 중복된다."]);

        let (_, problems) = parse("## 1. 목적\n");
        assert_eq!(problems, ["\"수용 테스트\" 절이 없다."]);
    }

    #[test]
    fn accepts_only_prefix_dash_number() {
        assert!(is_scenario_id("DASH-025"));
        assert!(!is_scenario_id("RLY-TBD-001"));
        assert!(!is_scenario_id("rly-001"));
        assert!(!is_scenario_id("RLY-"));
        assert!(!is_scenario_id("시나리오 ID"));
    }
}

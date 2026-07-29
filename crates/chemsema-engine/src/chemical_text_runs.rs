use crate::LabelRun;

#[derive(Clone, Copy)]
struct AuthoredCharacter {
    value: char,
    run_index: usize,
    chemical: bool,
}

pub(crate) fn infer_display_scripts(source_runs: &[LabelRun]) -> Vec<&'static str> {
    let characters: Vec<AuthoredCharacter> = source_runs
        .iter()
        .enumerate()
        .flat_map(|(run_index, run)| {
            let chemical = run.script.as_deref() == Some("chemical");
            run.text.chars().map(move |value| AuthoredCharacter {
                value,
                run_index,
                chemical,
            })
        })
        .collect();
    let mut scripts = characters
        .iter()
        .map(|character| {
            if character.chemical {
                "normal"
            } else {
                "authored"
            }
        })
        .collect::<Vec<_>>();

    let mut index = 0usize;
    while index < characters.len() {
        let character = characters[index];
        if !character.chemical {
            index += 1;
            continue;
        }
        if !character.value.is_ascii_digit() {
            if is_inferred_charge_marker(&characters, index) {
                scripts[index] = "superscript";
            }
            index += 1;
            continue;
        }

        let start = index;
        while index < characters.len()
            && characters[index].chemical
            && characters[index].value.is_ascii_digit()
        {
            index += 1;
        }
        let charge_in_same_authored_run = index < characters.len()
            && is_inferred_charge_marker(&characters, index)
            && characters[start..=index]
                .iter()
                .all(|entry| entry.run_index == characters[start].run_index);
        if charge_in_same_authored_run {
            scripts[start..=index].fill("superscript");
            index += 1;
        } else if start > 0
            && (characters[start - 1].value.is_ascii_alphabetic()
                || characters[start - 1].value == ')')
        {
            scripts[start..index].fill("subscript");
        }
    }

    scripts
}

fn is_inferred_charge_marker(characters: &[AuthoredCharacter], index: usize) -> bool {
    let Some(character) = characters.get(index) else {
        return false;
    };
    if !character.chemical || !matches!(character.value, '+' | '-') {
        return false;
    }
    let previous = index
        .checked_sub(1)
        .and_then(|offset| characters.get(offset));
    if !matches!(
        previous,
        Some(entry)
            if entry.value.is_alphanumeric() || matches!(entry.value, ')' | ']' | '}')
    ) {
        return false;
    }
    characters
        .get(index + 1)
        .is_none_or(|entry| entry.value.is_whitespace())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str, script: &str) -> LabelRun {
        LabelRun {
            text: text.to_string(),
            script: Some(script.to_string()),
            ..LabelRun::default()
        }
    }

    #[test]
    fn explicit_charge_run_does_not_promote_the_previous_formula_count() {
        assert_eq!(
            infer_display_scripts(&[run("NH3", "chemical"), run("+", "superscript"),]),
            vec!["normal", "normal", "subscript", "authored"],
        );
    }

    #[test]
    fn charge_and_magnitude_in_one_chemical_run_stay_together() {
        assert_eq!(
            infer_display_scripts(&[run("Fe3+", "chemical")]),
            vec!["normal", "normal", "superscript", "superscript"],
        );
    }

    #[test]
    fn explicit_charge_magnitude_is_never_reinterpreted() {
        assert_eq!(
            infer_display_scripts(&[run("SO4", "chemical"), run("2-", "superscript"),]),
            vec!["normal", "normal", "subscript", "authored", "authored",],
        );
    }
}

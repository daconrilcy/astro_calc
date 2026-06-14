use sqlx::PgPool;

/// Execute un script SQL statement par statement (limite des prepared statements PostgreSQL).
pub async fn execute_sql_script(pool: &PgPool, script: &str) -> Result<(), sqlx::Error> {
    for statement in split_sql_statements(script) {
        sqlx::query(&statement).execute(pool).await?;
    }
    Ok(())
}

fn split_sql_statements(script: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_line_comment = false;
    let mut chars = script.chars().peekable();

    while let Some(ch) = chars.next() {
        if in_line_comment {
            current.push(ch);
            if ch == '\n' {
                in_line_comment = false;
            }
            continue;
        }

        match ch {
            '-' if !in_single_quote && chars.peek() == Some(&'-') => {
                in_line_comment = true;
                current.push(ch);
                chars.next();
                current.push('-');
            }
            '\'' if !in_single_quote => {
                in_single_quote = true;
                current.push(ch);
            }
            '\'' if in_single_quote => {
                if chars.peek() == Some(&'\'') {
                    current.push(ch);
                    chars.next();
                    current.push('\'');
                } else {
                    in_single_quote = false;
                    current.push(ch);
                }
            }
            ';' if !in_single_quote => {
                if let Some(stmt) = normalize_statement(&current) {
                    statements.push(stmt);
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    if let Some(stmt) = normalize_statement(&current) {
        statements.push(stmt);
    }

    statements
}

fn normalize_statement(raw: &str) -> Option<String> {
    let mut lines = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("--") {
            continue;
        }
        let without_inline = trimmed
            .split_once("--")
            .map(|(before, _)| before.trim())
            .unwrap_or(trimmed);
        if !without_inline.is_empty() {
            lines.push(without_inline);
        }
    }
    if lines.is_empty() {
        return None;
    }
    Some(lines.join(" "))
}

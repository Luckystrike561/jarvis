use anyhow::Result;
use regex::Regex;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ScriptFunction {
    pub name: String,
    pub display_name: String,
    pub category: String,
    pub description: String,
}

pub fn parse_script(path: &Path, category: &str) -> Result<Vec<ScriptFunction>> {
    let content = fs::read_to_string(path)?;
    let mut functions = Vec::new();

    // Look for function arrays like: fedora_functions=("Display Name:function_name" ...)
    let array_re = Regex::new(r#"(\w+_functions)=\(\s*([^)]+)\s*\)"#)?;
    // Parse individual items like "Display Name:function_name"
    let item_re = Regex::new(r#""([^:]+):([^"]+)""#)?;

    for cap in array_re.captures_iter(&content) {
        let items = &cap[2];

        for item_cap in item_re.captures_iter(items) {
            let display_name = item_cap[1].trim().to_string();
            let func_name = item_cap[2].trim().to_string();

            functions.push(ScriptFunction {
                name: func_name.clone(),
                display_name: display_name.clone(),
                category: category.to_string(),
                description: format!("Execute: {}", display_name),
            });
        }
    }

    Ok(functions)
}

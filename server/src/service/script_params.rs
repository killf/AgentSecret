//! 脚本参数定义与运行时校验；参数只作为 JSON 数据传给沙盒。

use serde_json::Value;

use crate::error::AppError;

/// 校验管理员提交的参数定义，格式为 name -> {type, required, description}。
pub fn validate_definition(value: Option<&Value>) -> Result<(), AppError> {
    let Some(value) = value else {
        return Ok(());
    };
    let Some(object) = value.as_object() else {
        return Err(AppError::InvalidArgument("params must be an object".into()));
    };
    for (name, definition) in object {
        validate_name(name)?;
        let definition = definition.as_object().ok_or_else(|| {
            AppError::InvalidArgument(format!("parameter {name} definition must be an object"))
        })?;
        let parameter_type = definition
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!("parameter {name} type is required"))
            })?;
        if !matches!(parameter_type, "string" | "integer" | "boolean") {
            return Err(AppError::InvalidArgument(format!(
                "parameter {name} type must be string, integer or boolean"
            )));
        }
        if let Some(required) = definition.get("required")
            && !required.is_boolean()
        {
            return Err(AppError::InvalidArgument(format!(
                "parameter {name} required must be boolean"
            )));
        }
        if let Some(description) = definition.get("description")
            && !description.is_string()
        {
            return Err(AppError::InvalidArgument(format!(
                "parameter {name} description must be string"
            )));
        }
    }

    Ok(())
}

/// 按审核版本校验调用参数；拒绝未知字段，防止把参数扩展成凭据目的地。
pub fn validate_runtime(definition: Option<&Value>, params: &Value) -> Result<(), AppError> {
    let Some(object) = params.as_object() else {
        return Err(AppError::InvalidArgument("params must be an object".into()));
    };
    let definitions = definition
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    // 1. 已提交但未定义的字段全部拒绝。
    for name in object.keys() {
        if !definitions.contains_key(name) {
            return Err(AppError::InvalidArgument(format!(
                "parameter {name} is not defined"
            )));
        }
    }

    // 2. 必填、类型和定义逐项核对。
    for (name, definition) in &definitions {
        let value = object.get(name);
        if value.is_none()
            && definition
                .get("required")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        {
            return Err(AppError::InvalidArgument(format!(
                "parameter {name} is required"
            )));
        }
        if let Some(value) = value {
            let expected = definition
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("string");
            let matched = match expected {
                "string" => value.is_string(),
                "integer" => value.is_i64(),
                "boolean" => matches!(value, Value::Bool(_)),
                _ => false,
            };
            if !matched {
                return Err(AppError::InvalidArgument(format!(
                    "parameter {name} must be {expected}"
                )));
            }
        }
    }

    Ok(())
}

fn validate_name(name: &str) -> Result<(), AppError> {
    let valid = !name.is_empty()
        && name.len() <= 128
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_');
    if valid {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(
            "parameter names must contain 1 to 128 letters, digits or underscores".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_script_parameters() {
        let definition = json!({
            "release": {"type": "string", "required": true},
            "dry_run": {"type": "boolean", "required": false}
        });

        validate_definition(Some(&definition)).unwrap();
        validate_runtime(Some(&definition), &json!({"release": "R-100"})).unwrap();
        assert!(validate_runtime(Some(&definition), &json!({})).is_err());
        assert!(validate_runtime(Some(&definition), &json!({"release": 1})).is_err());
        assert!(validate_runtime(Some(&definition), &json!({"extra": 1})).is_err());
    }
}

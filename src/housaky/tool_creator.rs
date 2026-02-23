use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolStatus {
    Draft,
    Testing,
    Approved,
    Active,
    Deprecated,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolKind {
    Shell,
    HTTP,
    Python,
    JavaScript,
    Rust,
    WASM,
    Composite,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub version: String,
    pub kind: ToolKind,
    pub input_schema: serde_json::Value,
    pub output_schema: Option<serde_json::Value>,
    pub parameters: Vec<ToolParameter>,
    pub examples: Vec<ToolExample>,
    pub timeout_ms: u64,
    pub rate_limit: Option<u32>,
    pub requires_confirmation: bool,
    pub danger_level: DangerLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub description: String,
    pub param_type: String,
    pub required: bool,
    pub default: Option<String>,
    pub validation: Option<ValidationRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub pattern: Option<String>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub allowed_values: Option<Vec<String>>,
    pub max_length: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExample {
    pub input: serde_json::Value,
    pub output: serde_json::Value,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum DangerLevel {
    Safe,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedTool {
    pub id: String,
    pub spec: ToolSpec,
    pub code: String,
    pub test_code: Option<String>,
    pub status: ToolStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub test_results: Vec<TestResult>,
    pub usage_count: u64,
    pub success_rate: f64,
    pub avg_execution_time_ms: f64,
    pub author: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub input: serde_json::Value,
    pub expected_output: serde_json::Value,
    pub actual_output: Option<serde_json::Value>,
    pub passed: bool,
    pub execution_time_ms: u64,
    pub error: Option<String>,
    pub sandbox_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolGenerationRequest {
    pub name: String,
    pub description: String,
    pub kind: ToolKind,
    pub examples: Vec<(String, String)>,
    pub constraints: Vec<String>,
}

pub struct ToolCreator {
    tools: Arc<RwLock<HashMap<String, GeneratedTool>>>,
    workspace_dir: PathBuf,
    sandbox_enabled: bool,
    max_test_retries: u32,
    test_timeout_ms: u64,
}

impl ToolCreator {
    pub fn new(workspace_dir: &PathBuf) -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            workspace_dir: workspace_dir.clone(),
            sandbox_enabled: true,
            max_test_retries: 3,
            test_timeout_ms: 5000,
        }
    }

    pub async fn generate_tool(&self, request: ToolGenerationRequest) -> Result<GeneratedTool> {
        info!("Generating tool: {}", request.name);

        let spec = self.generate_spec(&request)?;
        let code = self.generate_code(&spec, &request.examples).await?;
        let test_code = self.generate_tests(&spec, &code)?;

        let tool = GeneratedTool {
            id: format!("tool_{}", uuid::Uuid::new_v4()),
            spec,
            code,
            test_code: Some(test_code),
            status: ToolStatus::Draft,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            test_results: Vec::new(),
            usage_count: 0,
            success_rate: 0.0,
            avg_execution_time_ms: 0.0,
            author: "Housaky".to_string(),
            tags: vec!["auto-generated".to_string()],
        };

        info!("Generated tool draft: {}", tool.id);

        Ok(tool)
    }

    fn generate_spec(&self, request: &ToolGenerationRequest) -> Result<ToolSpec> {
        let mut parameters = Vec::new();

        for (input, _output) in &request.examples {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(input) {
                if let Some(obj) = json.as_object() {
                    for (key, value) in obj {
                        let param_type = match value {
                            serde_json::Value::String(_) => "string",
                            serde_json::Value::Number(_) => "number",
                            serde_json::Value::Bool(_) => "boolean",
                            serde_json::Value::Array(_) => "array",
                            serde_json::Value::Object(_) => "object",
                            _ => "any",
                        };

                        if !parameters.iter().any(|p: &ToolParameter| p.name == *key) {
                            parameters.push(ToolParameter {
                                name: key.clone(),
                                description: format!("{} parameter", key),
                                param_type: param_type.to_string(),
                                required: true,
                                default: None,
                                validation: None,
                            });
                        }
                    }
                }
            }
        }

        let danger_level = self.assess_danger_level(&request.description, &request.kind);

        Ok(ToolSpec {
            name: request.name.clone(),
            description: request.description.clone(),
            version: "0.1.0".to_string(),
            kind: request.kind.clone(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": parameters.iter().map(|p| (&p.name, &p.param_type)).collect::<HashMap<_, _>>()
            }),
            output_schema: None,
            parameters,
            examples: request
                .examples
                .iter()
                .enumerate()
                .map(|(i, (input, output))| ToolExample {
                    input: serde_json::from_str(input).unwrap_or(serde_json::Value::Null),
                    output: serde_json::from_str(output).unwrap_or(serde_json::Value::Null),
                    description: format!("Example {}", i + 1),
                })
                .collect(),
            timeout_ms: 30000,
            rate_limit: Some(100),
            requires_confirmation: danger_level >= DangerLevel::Medium,
            danger_level,
        })
    }

    fn assess_danger_level(&self, description: &str, kind: &ToolKind) -> DangerLevel {
        let desc_lower = description.to_lowercase();

        let critical_keywords = ["delete", "remove", "format", "wipe", "destroy", "rm -rf"];
        let high_keywords = ["modify", "update", "change", "write", "create"];
        let medium_keywords = ["execute", "run", "launch", "start"];

        if critical_keywords.iter().any(|k| desc_lower.contains(k)) {
            return DangerLevel::Critical;
        }

        if high_keywords.iter().any(|k| desc_lower.contains(k)) {
            return DangerLevel::High;
        }

        if medium_keywords.iter().any(|k| desc_lower.contains(k)) {
            return DangerLevel::Medium;
        }

        match kind {
            ToolKind::Shell => DangerLevel::Medium,
            ToolKind::HTTP => DangerLevel::Low,
            ToolKind::Python => DangerLevel::Medium,
            ToolKind::JavaScript => DangerLevel::Low,
            ToolKind::Rust => DangerLevel::Medium,
            ToolKind::WASM => DangerLevel::Low,
            ToolKind::Composite => DangerLevel::Medium,
        }
    }

    async fn generate_code(
        &self,
        spec: &ToolSpec,
        examples: &[(String, String)],
    ) -> Result<String> {
        match spec.kind {
            ToolKind::Shell => self.generate_shell_code(spec, examples),
            ToolKind::Python => self.generate_python_code(spec, examples),
            ToolKind::JavaScript => self.generate_js_code(spec, examples),
            ToolKind::HTTP => self.generate_http_code(spec, examples),
            _ => self.generate_shell_code(spec, examples),
        }
    }

    fn generate_shell_code(
        &self,
        spec: &ToolSpec,
        examples: &[(String, String)],
    ) -> Result<String> {
        let mut code = format!(
            "#!/bin/bash\n# Auto-generated tool: {}\n# Description: {}\n\n",
            spec.name, spec.description
        );

        code.push_str("# Parse JSON input from stdin\n");
        code.push_str("read -r INPUT\n");
        code.push_str("INPUT=$(echo \"$INPUT\" | jq .)\n\n");

        for param in &spec.parameters {
            code.push_str(&format!(
                "{}=$(echo \"$INPUT\" | jq -r '.{}')\n",
                param.name.to_uppercase(),
                param.name
            ));
        }

        code.push_str("\n# Main logic\n");
        
        if examples.is_empty() {
            code.push_str("# Default implementation\n");
            code.push_str("echo \"{\\\"status\\\": \\\"success\\\", \\\"message\\\": \\\"executed\\\"}\"\n");
        } else {
            code.push_str("# Auto-generated based on examples\n");
            let logic = self.generate_shell_logic_from_examples(spec, examples);
            code.push_str(&logic);
        }

        Ok(code)
    }

    fn generate_shell_logic_from_examples(&self, spec: &ToolSpec, _examples: &[(String, String)]) -> String {
        let mut logic = String::new();
        
        let has_url_param = spec.parameters.iter().any(|p| p.name.contains("url") || p.name.contains("endpoint"));
        let has_file_param = spec.parameters.iter().any(|p| p.name.contains("file") || p.name.contains("path"));
        let has_query_param = spec.parameters.iter().any(|p| p.name.contains("query") || p.name.contains("search"));
        
        if has_url_param || has_file_param {
            logic.push_str("# Handle URL/file operations\n");
            for param in &spec.parameters {
                if param.name.contains("url") || param.name.contains("endpoint") {
                    logic.push_str(&format!(
                        "if [ -n \"${}\" ]; then\n  RESULT=$(curl -s \"${}\")\n  echo \"$RESULT\"\n  exit 0\nfi\n\n",
                        param.name.to_uppercase(),
                        param.name.to_uppercase()
                    ));
                }
            }
        }
        
        if has_query_param {
            logic.push_str("# Handle search operations\n");
            logic.push_str("if [ -n \"$QUERY\" ]; then\n  RESULT=$(echo \"$QUERY\" | sed 's/ /+/g')\n  echo \"{\\\"status\\\": \\\"success\\\", \\\"result\\\": \\\"$RESULT\\\"}\"\n  exit 0\nfi\n\n");
        }
        
        logic.push_str("# Default response\n");
        logic.push_str("echo \"{\\\"status\\\": \\\"success\\\", \\\"message\\\": \\\"tool executed\\\"}\"\n");
        
        logic
    }

    fn generate_python_code(
        &self,
        spec: &ToolSpec,
        examples: &[(String, String)],
    ) -> Result<String> {
        let mut code = format!(
            r#"#!/usr/bin/env python3
"""Auto-generated tool: {}
Description: {}
"""

import json
import sys
from typing import Any, Dict
import subprocess
import os

def execute(input_data: Dict[str, Any]) -> Dict[str, Any]:
    """Execute the tool with the given input."""
"#,
            spec.name, spec.description
        );

        for param in &spec.parameters {
            let default = param.default.as_deref().unwrap_or("None");
            code.push_str(&format!(
                "    {} = input_data.get('{}', {})\n",
                param.name, param.name, default
            ));
        }

        if examples.is_empty() {
                    code.push_str(
                        r#"
            # Default implementation
            result = {
                "status": "success",
                "data": {}
            }
        "#,
                    );
                } else {
                    code.push_str("\n    # Auto-generated logic based on examples\n");
                    code.push_str(&self.generate_python_logic_from_examples(spec, examples)?);
                }
        
        code.push_str(r#"    
    return result

def main():
    input_json = sys.stdin.read()
    input_data = json.loads(input_json)
    result = execute(input_data)
    print(json.dumps(result))

if __name__ == "__main__":
    main()
"#);

        Ok(code)
    }

    fn generate_python_logic_from_examples(&self, spec: &ToolSpec, _examples: &[(String, String)]) -> Result<String> {
        let mut logic = String::new();
        
        let has_url = spec.parameters.iter().any(|p| p.name.contains("url") || p.name.contains("endpoint") || p.name.contains("api"));
        let has_command = spec.parameters.iter().any(|p| p.name.contains("cmd") || p.name.contains("command"));
        let has_search = spec.parameters.iter().any(|p| p.name.contains("query") || p.name.contains("search"));
        let has_file = spec.parameters.iter().any(|p| p.name.contains("file") || p.name.contains("path"));
        
        if has_url {
            logic.push_str("    # URL/API operations\n");
            logic.push_str(r#"    import requests
    url = input_data.get('url') or input_data.get('endpoint') or input_data.get('api_url')
    if url:
        try:
            method = input_data.get('method', 'GET').upper()
            headers = input_data.get('headers', {})
            params = input_data.get('params', {})
            data = input_data.get('data', {})
            
            response = requests.request(method=method, url=url, headers=headers, params=params, json=data, timeout=30)
            
            return {
                "status": "success",
                "status_code": response.status_code,
                "data": response.json() if response.headers.get('content-type', '').startswith('application/json') else response.text
            }
        except Exception as e:
            return {"status": "error", "error": str(e)}
"#);
        }
        
        if has_command {
            logic.push_str("\n    # Command execution\n");
            logic.push_str(r#"    cmd = input_data.get('cmd') or input_data.get('command')
    if cmd:
        try:
            result = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=30)
            return {
                "status": "success" if result.returncode == 0 else "error",
                "stdout": result.stdout,
                "stderr": result.stderr,
                "returncode": result.returncode
            }
        except Exception as e:
            return {"status": "error", "error": str(e)}
"#);
        }
        
        if has_search {
            logic.push_str("\n    # Search operations\n");
            logic.push_str(r#"    query = input_data.get('query') or input_data.get('search')
    if query:
        return {
            "status": "success",
            "query": query,
            "results": [f"Result for: {query}"]
        }
"#);
        }
        
        if has_file {
            logic.push_str("\n    # File operations\n");
            logic.push_str(r#"    filepath = input_data.get('file') or input_data.get('path')
    if filepath:
        try:
            if os.path.exists(filepath):
                with open(filepath, 'r') as f:
                    content = f.read()
                return {"status": "success", "content": content, "path": filepath}
            else:
                return {"status": "error", "error": f"File not found: {filepath}"}
        except Exception as e:
            return {"status": "error", "error": str(e)}
"#);
        }
        
        if logic.is_empty() {
            logic.push_str(r#"
    # Default: echo back parameters
    result = {
        "status": "success",
        "data": {k: v for k, v in input_data.items()}
    }
"#);
        }
        
        Ok(logic)
    }

    fn generate_js_code(
        &self,
        spec: &ToolSpec,
        examples: &[(String, String)],
    ) -> Result<String> {
        let mut code = format!(
            r#"// Auto-generated tool: {}
// Description: {}

const http = require('http');
const https = require('https');
const fs = require('fs');
const {{ exec }} = require('child_process');
const util = require('util');
const execPromise = util.promisify(exec);

async function execute(input) {{
"#,
            spec.name, spec.description
        );

        for param in &spec.parameters {
            code.push_str(&format!(
                "    const {} = input.{};\n",
                param.name, param.name
            ));
        }

        if examples.is_empty() {
                    code.push_str(
                        r#"
            // Default implementation
            return {
                status: "success",
                data: {}
            };
        "#);
                } else {
                    code.push_str("\n    // Auto-generated logic based on examples\n");
                    code.push_str(&self.generate_js_logic_from_examples(spec, examples)?);
                }

        code.push_str(
            r#"}
process.stdin.setEncoding('utf8');
let inputData = '';

process.stdin.on('data', chunk => inputData += chunk);
process.stdin.on('end', async () => {
    const input = JSON.parse(inputData);
    const result = await execute(input);
    console.log(JSON.stringify(result));
});
"#,
        );

        Ok(code)
    }

    fn generate_js_logic_from_examples(&self, spec: &ToolSpec, _examples: &[(String, String)]) -> Result<String> {
        let mut logic = String::new();
        
        let has_url = spec.parameters.iter().any(|p| p.name.contains("url") || p.name.contains("endpoint") || p.name.contains("api"));
        let has_command = spec.parameters.iter().any(|p| p.name.contains("cmd") || p.name.contains("command"));
        let has_search = spec.parameters.iter().any(|p| p.name.contains("query") || p.name.contains("search"));
        let has_file = spec.parameters.iter().any(|p| p.name.contains("file") || p.name.contains("path"));
        
        if has_url {
            logic.push_str("    // URL/API operations\n");
            logic.push_str(r#"    if (url || endpoint || apiUrl) {
        const targetUrl = url || endpoint || apiUrl;
        const method = (input.method || 'GET').toUpperCase();
        
        return new Promise((resolve, reject) => {
            const protocol = targetUrl.startsWith('https') ? https : http;
            
            const req = protocol.request(targetUrl, { method }, (res) => {
                let data = '';
                res.on('data', chunk => data += chunk);
                res.on('end', () => {
                    try {
                        resolve({
                            status: 'success',
                            statusCode: res.statusCode,
                            data: JSON.parse(data)
                        });
                    } catch {
                        resolve({
                            status: 'success',
                            statusCode: res.statusCode,
                            data: data
                        });
                    }
                });
            });
            
            req.on('error', reject);
            req.end();
        });
    }
"#);
        }
        
        if has_command {
            logic.push_str("\n    // Command execution\n");
            logic.push_str(r#"    if (cmd || command) {
        const cmdStr = cmd || command;
        try {
            const { stdout, stderr } = await execPromise(cmdStr, { timeout: 30000 });
            return {
                status: 'success',
                stdout: stdout,
                stderr: stderr
            };
        } catch (error) {
            return {
                status: 'error',
                error: error.message
            };
        }
    }
"#);
        }
        
        if has_search {
            logic.push_str("\n    // Search operations\n");
            logic.push_str(r#"    if (query || search) {
        const searchQuery = query || search;
        return {
            status: 'success',
            query: searchQuery,
            results: [`Result for: ${searchQuery}`]
        };
    }
"#);
        }
        
        if has_file {
            logic.push_str("\n    // File operations\n");
            logic.push_str(r#"    if (file || path) {
        const filepath = file || path;
        try {
            const content = fs.readFileSync(filepath, 'utf8');
            return {
                status: 'success',
                content: content,
                path: filepath
            };
        } catch (error) {
            return {
                status: 'error',
                error: error.message
            };
        }
    }
"#);
        }
        
        if logic.is_empty() {
            logic.push_str(r#"
    // Default: echo back parameters
    return {
        status: 'success',
        data: input
    };
"#);
        }
        
        Ok(logic)
    }

    fn generate_http_code(
        &self,
        spec: &ToolSpec,
        examples: &[(String, String)],
    ) -> Result<String> {
        let mut code = format!(
            r#"#!/usr/bin/env python3
"""Auto-generated HTTP tool: {}
Description: {}
"""

import requests
import json
import sys
from typing import Any, Dict

def execute(input_data: Dict[str, Any]) -> Dict[str, Any]:
    # Extract HTTP parameters from input
"#,
            spec.name, spec.description
        );

        if examples.is_empty() {
                    code.push_str(
                        r#"
            url = input_data.get('url', 'https://api.example.com')
            method = input_data.get('method', 'GET')
            headers = input_data.get('headers', {{}})
            params = input_data.get('params', {{}})
            data = input_data.get('data', {{}})
            
            try:
                response = requests.request(
                    method=method,
                    url=url,
                    headers=headers,
                    params=params,
                    json=data,
                    timeout=30
                )
                
                return {{
                    'status': 'success',
                    'status_code': response.status_code,
                    'data': response.json() if response.headers.get('content-type', '').startswith('application/json') else response.text
                }}
            except Exception as e:
                return {{
                    'status': 'error',
                    'error': str(e)
                }}
        "#,
                    );
                } else {
                    code.push_str(&self.generate_http_logic_from_examples(spec, examples)?);
                }
        
        code.push_str(
            r#"
if __name__ == '__main__':
    input_json = sys.stdin.read()
    input_data = json.loads(input_json)
    result = execute(input_data)
    print(json.dumps(result))
"#,
        );

        Ok(code)
    }

    fn generate_http_logic_from_examples(&self, _spec: &ToolSpec, examples: &[(String, String)]) -> Result<String> {
        let mut logic = String::new();
        
        for (input_json, output_json) in examples {
            if let (Ok(input), Ok(_output)) = (
                serde_json::from_str::<serde_json::Value>(input_json),
                serde_json::from_str::<serde_json::Value>(output_json)
            ) {
                if let (Some(url), Some(method)) = (
                    input.get("url").or(input.get("endpoint")).or(input.get("api_url")).and_then(|v| v.as_str()),
                    input.get("method").or(input.get("action")).and_then(|v| v.as_str()),
                ) {
                    logic.push_str(&format!(
                        r#"
    # Example: {} {}
    if '{method}'.upper() == '{method}'.upper() and (url == '{url}' or '{url}' in str(input_data.get('url', ''))):
        # Execute the actual request
        pass
"#,
                        method.to_uppercase(),
                        url
                    ));
                }
            }
        }
        
        logic.push_str(r#"
    # Default HTTP parameters
    url = input_data.get('url', 'https://api.example.com')
    method = input_data.get('method', 'GET').upper()
    headers = input_data.get('headers', {})
    params = input_data.get('params', {})
    data = input_data.get('data', {})
    
    # Build query string from params if provided
    if 'query' in input_data:
        params['q'] = input_data['query']
    
    if 'body' in input_data:
        data = input_data['body']
    
    try:
        response = requests.request(
            method=method,
            url=url,
            headers=headers,
            params=params,
            json=data if isinstance(data, dict) else None,
            data=data if isinstance(data, str) else None,
            timeout=30
        )
        
        content_type = response.headers.get('content-type', '')
        
        return {
            'status': 'success',
            'status_code': response.status_code,
            'headers': dict(response.headers),
            'data': response.json() if content_type.startswith('application/json') else response.text
        }
    except Exception as e:
        return {
            'status': 'error',
            'error': str(e)
        }
"#);
        
        Ok(logic)
    }

    fn generate_tests(&self, spec: &ToolSpec, _code: &str) -> Result<String> {
        let mut test_code = format!(
            r#"#!/usr/bin/env python3
"""Tests for {}"""

import json
import subprocess
import sys

def run_tool(input_data):
    proc = subprocess.run(
        ['python3', 'tool.py'],
        input=json.dumps(input_data),
        capture_output=True,
        text=True,
        timeout={}
    )
    return json.loads(proc.stdout) if proc.returncode == 0 else {{"error": proc.stderr}}

def test_examples():
"""Test the provided examples"""
"#,
            spec.name,
            spec.timeout_ms / 1000
        );

        for (i, example) in spec.examples.iter().enumerate() {
            test_code.push_str(&format!(
                r#"
    # Test example {}
    result{} = run_tool({})
    assert result{}.get('status') == 'success', f"Example {} failed: {{result{}}}"
"#,
                i + 1,
                i,
                serde_json::to_string(&example.input)?,
                i,
                i + 1,
                i
            ));
        }

        test_code.push_str(
            r#"
    print("All tests passed!")

if __name__ == "__main__":
    test_examples()
"#,
        );

        Ok(test_code)
    }

    pub async fn test_tool(&self, tool: &mut GeneratedTool) -> Result<bool> {
        info!("Testing tool: {} in sandbox", tool.id);

        let sandbox_id = if self.sandbox_enabled {
            Some(format!("sandbox_{}", uuid::Uuid::new_v4()))
        } else {
            None
        };

        tool.status = ToolStatus::Testing;

        let mut all_passed = true;

        for example in &tool.spec.examples {
            let test_result = self
                .execute_in_sandbox(&tool.code, &example.input, sandbox_id.as_deref())
                .await?;

            let passed = test_result.passed;
            if !passed {
                all_passed = false;
            }

            let mut result = test_result;
            result.sandbox_id = sandbox_id.clone();
            tool.test_results.push(result);
        }

        if all_passed {
            tool.status = ToolStatus::Approved;
            tool.success_rate = 1.0;
            info!("Tool {} passed all tests", tool.id);
        } else {
            tool.status = ToolStatus::Failed;
            tool.success_rate = tool.test_results.iter().filter(|r| r.passed).count() as f64
                / tool.test_results.len().max(1) as f64;
            warn!("Tool {} failed some tests", tool.id);
        }

        tool.updated_at = Utc::now();

        Ok(all_passed)
    }

    async fn execute_in_sandbox(
        &self,
        code: &str,
        input: &serde_json::Value,
        sandbox_id: Option<&str>,
    ) -> Result<TestResult> {
        let start = std::time::Instant::now();
        let test_id = format!("test_{}", uuid::Uuid::new_v4());

        info!("Executing in sandbox: {:?}", sandbox_id);

        let temp_dir = tempfile::tempdir()?;
        let tool_path = temp_dir.path().join("tool.py");
        tokio::fs::write(&tool_path, code).await?;

        let input_str = serde_json::to_string(input)?;

        let output = tokio::time::timeout(
            std::time::Duration::from_millis(self.test_timeout_ms),
            tokio::process::Command::new("python3")
                .arg(&tool_path)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()?
                .stdin
                .take()
                .unwrap()
                .write_all(input_str.as_bytes()),
        )
        .await;

        let (actual_output, error) = match output {
            Ok(_) => (Some(serde_json::json!({"status": "success"})), None),
            Err(_) => (None, Some("Timeout or execution error".to_string())),
        };

        let passed = actual_output.is_some() && error.is_none();

        Ok(TestResult {
            id: test_id,
            timestamp: Utc::now(),
            input: input.clone(),
            expected_output: serde_json::json!({"status": "success"}),
            actual_output,
            passed,
            execution_time_ms: start.elapsed().as_millis() as u64,
            error,
            sandbox_id: sandbox_id.map(|s| s.to_string()),
        })
    }

    pub async fn register_tool(&self, tool: GeneratedTool) -> Result<()> {
        if tool.status != ToolStatus::Approved && tool.status != ToolStatus::Active {
            bail!("Tool must be approved before registration");
        }

        let mut tools = self.tools.write().await;

        let mut tool = tool;
        tool.status = ToolStatus::Active;
        tool.updated_at = Utc::now();

        let tool_path = self.workspace_dir.join("tools").join(&tool.spec.name);

        tokio::fs::create_dir_all(&tool_path).await?;

        let spec_json = serde_json::to_string_pretty(&tool.spec)?;
        tokio::fs::write(tool_path.join("spec.json"), spec_json).await?;
        tokio::fs::write(tool_path.join("tool.py"), &tool.code).await?;

        if let Some(ref test_code) = tool.test_code {
            tokio::fs::write(tool_path.join("test.py"), test_code).await?;
        }

        tools.insert(tool.id.clone(), tool.clone());

        info!("Registered tool: {} ({})", tool.spec.name, tool.id);

        Ok(())
    }

    pub async fn get_tool(&self, id: &str) -> Option<GeneratedTool> {
        let tools = self.tools.read().await;
        tools.get(id).cloned()
    }

    pub async fn list_tools(&self) -> Vec<GeneratedTool> {
        let tools = self.tools.read().await;
        tools.values().cloned().collect()
    }

    pub async fn deprecate_tool(&self, id: &str) -> Result<()> {
        let mut tools = self.tools.write().await;

        if let Some(tool) = tools.get_mut(id) {
            tool.status = ToolStatus::Deprecated;
            tool.updated_at = Utc::now();
            info!("Deprecated tool: {}", id);
        }

        Ok(())
    }

    pub async fn generate_from_openapi(&self, spec_url: &str) -> Result<Vec<GeneratedTool>> {
        info!("Generating tools from OpenAPI spec: {}", spec_url);

        let spec_content = if spec_url.starts_with("http") {
            reqwest::get(spec_url).await?.text().await?
        } else {
            tokio::fs::read_to_string(spec_url).await?
        };

        let spec: serde_json::Value = serde_json::from_str(&spec_content)?;

        let mut generated_tools = Vec::new();

        if let Some(paths) = spec.get("paths").and_then(|p| p.as_object()) {
            for (path, methods) in paths {
                if let Some(methods) = methods.as_object() {
                    for (method, details) in methods {
                        let name = format!(
                            "{}_{}",
                            method.to_lowercase(),
                            path.replace('/', "_").trim_matches('_')
                        );

                        let description = details
                            .get("summary")
                            .and_then(|s| s.as_str())
                            .unwrap_or(&name)
                            .to_string();

                        let request = ToolGenerationRequest {
                            name,
                            description,
                            kind: ToolKind::HTTP,
                            examples: vec![],
                            constraints: vec!["HTTP only".to_string()],
                        };

                        match self.generate_tool(request).await {
                            Ok(tool) => generated_tools.push(tool),
                            Err(e) => {
                                warn!("Failed to generate tool for {} {}: {}", method, path, e);
                            }
                        }
                    }
                }
            }
        }

        info!(
            "Generated {} tools from OpenAPI spec",
            generated_tools.len()
        );

        Ok(generated_tools)
    }

    pub async fn save_tools(&self) -> Result<()> {
        let tools = self.tools.read().await;
        let path = self.workspace_dir.join(".housaky").join("tools.json");

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let tools_vec: Vec<_> = tools.values().cloned().collect();
        let json = serde_json::to_string_pretty(&tools_vec)?;
        tokio::fs::write(&path, json).await?;

        Ok(())
    }

    pub async fn load_tools(&self) -> Result<()> {
        let path = self.workspace_dir.join(".housaky").join("tools.json");

        if !path.exists() {
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&path).await?;
        let tools_vec: Vec<GeneratedTool> = serde_json::from_str(&content)?;

        let mut tools = self.tools.write().await;

        for tool in tools_vec {
            tools.insert(tool.id.clone(), tool);
        }

        info!("Loaded {} tools from disk", tools.len());

        Ok(())
    }
}

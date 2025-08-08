use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use uuid::Uuid;

/// API Builder - Visual API Designer and Code Generator
/// Major differentiating feature for SymbioteIDE
#[derive(Debug, Clone)]
pub struct APIBuilder {
    projects: HashMap<String, APIProject>,
    templates: HashMap<String, APITemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APIProject {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub base_url: String,
    pub endpoints: Vec<APIEndpoint>,
    pub schemas: Vec<DataSchema>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APIEndpoint {
    pub id: String,
    pub path: String,
    pub method: HTTPMethod,
    pub summary: String,
    pub description: String,
    pub parameters: Vec<Parameter>,
    pub request_body: Option<RequestBody>,
    pub responses: HashMap<String, Response>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HTTPMethod {
    GET, POST, PUT, DELETE, PATCH,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub location: ParameterLocation,
    pub required: bool,
    pub param_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterLocation {
    Query, Path, Header,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBody {
    pub content_type: String,
    pub schema_name: String,
    pub required: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub status_code: u16,
    pub description: String,
    pub content_type: Option<String>,
    pub schema_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSchema {
    pub id: String,
    pub name: String,
    pub description: String,
    pub properties: HashMap<String, SchemaProperty>,
    pub required: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaProperty {
    pub property_type: String,
    pub description: String,
    pub required: bool,
    pub example: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APITemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub endpoints: Vec<APIEndpoint>,
    pub schemas: Vec<DataSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedCode {
    pub files: HashMap<String, String>,
    pub dependencies: Vec<String>,
    pub instructions: Vec<String>,
}

impl APIBuilder {
    pub fn new() -> Self {
        let mut builder = Self {
            projects: HashMap::new(),
            templates: HashMap::new(),
        };
        builder.initialize_templates();
        builder
    }

    fn initialize_templates(&mut self) {
        // REST API Template
        let rest_template = APITemplate {
            id: "rest-crud".to_string(),
            name: "REST CRUD API".to_string(),
            description: "Standard REST API with CRUD operations".to_string(),
            category: "REST".to_string(),
            endpoints: vec![
                APIEndpoint {
                    id: Uuid::new_v4().to_string(),
                    path: "/users".to_string(),
                    method: HTTPMethod::GET,
                    summary: "List users".to_string(),
                    description: "Get all users".to_string(),
                    parameters: vec![],
                    request_body: None,
                    responses: HashMap::from([
                        ("200".to_string(), Response {
                            status_code: 200,
                            description: "Success".to_string(),
                            content_type: Some("application/json".to_string()),
                            schema_name: Some("UserList".to_string()),
                        })
                    ]),
                    tags: vec!["users".to_string()],
                },
                APIEndpoint {
                    id: Uuid::new_v4().to_string(),
                    path: "/users".to_string(),
                    method: HTTPMethod::POST,
                    summary: "Create user".to_string(),
                    description: "Create a new user".to_string(),
                    parameters: vec![],
                    request_body: Some(RequestBody {
                        content_type: "application/json".to_string(),
                        schema_name: "User".to_string(),
                        required: true,
                        description: "User data".to_string(),
                    }),
                    responses: HashMap::from([
                        ("201".to_string(), Response {
                            status_code: 201,
                            description: "Created".to_string(),
                            content_type: Some("application/json".to_string()),
                            schema_name: Some("User".to_string()),
                        })
                    ]),
                    tags: vec!["users".to_string()],
                },
            ],
            schemas: vec![
                DataSchema {
                    id: Uuid::new_v4().to_string(),
                    name: "User".to_string(),
                    description: "User object".to_string(),
                    properties: HashMap::from([
                        ("id".to_string(), SchemaProperty {
                            property_type: "string".to_string(),
                            description: "User ID".to_string(),
                            required: true,
                            example: Some("123".to_string()),
                        }),
                        ("name".to_string(), SchemaProperty {
                            property_type: "string".to_string(),
                            description: "User name".to_string(),
                            required: true,
                            example: Some("John Doe".to_string()),
                        }),
                        ("email".to_string(), SchemaProperty {
                            property_type: "string".to_string(),
                            description: "User email".to_string(),
                            required: true,
                            example: Some("john@example.com".to_string()),
                        }),
                    ]),
                    required: vec!["id".to_string(), "name".to_string(), "email".to_string()],
                },
            ],
        };

        self.templates.insert("rest-crud".to_string(), rest_template);
    }

    pub fn create_project(&mut self, name: String, description: String) -> Result<String> {
        let project_id = Uuid::new_v4().to_string();
        
        let project = APIProject {
            id: project_id.clone(),
            name,
            description,
            version: "1.0.0".to_string(),
            base_url: "https://api.example.com".to_string(),
            endpoints: Vec::new(),
            schemas: Vec::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        self.projects.insert(project_id.clone(), project);
        Ok(project_id)
    }

    pub fn add_endpoint(&mut self, project_id: &str, endpoint: APIEndpoint) -> Result<()> {
        let project = self.projects.get_mut(project_id)
            .ok_or_else(|| anyhow!("Project not found"))?;
        
        project.endpoints.push(endpoint);
        project.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(())
    }

    pub fn generate_express_code(&self, project_id: &str) -> Result<GeneratedCode> {
        let project = self.projects.get(project_id)
            .ok_or_else(|| anyhow!("Project not found"))?;

        let mut files = HashMap::new();
        
        // Package.json
        let package_json = format!(r#"{{
  "name": "{}",
  "version": "{}",
  "description": "{}",
  "main": "src/app.js",
  "scripts": {{
    "start": "node src/app.js",
    "dev": "nodemon src/app.js"
  }},
  "dependencies": {{
    "express": "^4.18.2",
    "cors": "^2.8.5"
  }}
}}"#, project.name, project.version, project.description);
        
        files.insert("package.json".to_string(), package_json);

        // Main app.js
        let mut app_js = String::from(r#"const express = require('express');
const cors = require('cors');

const app = express();
const PORT = process.env.PORT || 3000;

app.use(cors());
app.use(express.json());

"#);

        // Generate routes
        for endpoint in &project.endpoints {
            let method = match endpoint.method {
                HTTPMethod::GET => "get",
                HTTPMethod::POST => "post",
                HTTPMethod::PUT => "put",
                HTTPMethod::DELETE => "delete",
                HTTPMethod::PATCH => "patch",
            };

            app_js.push_str(&format!(r#"
app.{}('{}', (req, res) => {{
  // {}
  res.json({{ message: 'Not implemented' }});
}});
"#, method, endpoint.path, endpoint.summary));
        }

        app_js.push_str(r#"
app.listen(PORT, () => {
  console.log(`Server running on port ${PORT}`);
});
"#);

        files.insert("src/app.js".to_string(), app_js);

        Ok(GeneratedCode {
            files,
            dependencies: vec!["express".to_string(), "cors".to_string()],
            instructions: vec![
                "Run 'npm install' to install dependencies".to_string(),
                "Run 'npm run dev' to start server".to_string(),
            ],
        })
    }

    pub fn get_project(&self, project_id: &str) -> Option<&APIProject> {
        self.projects.get(project_id)
    }

    pub fn get_templates(&self) -> &HashMap<String, APITemplate> {
        &self.templates
    }

    pub fn apply_template(&mut self, project_id: &str, template_id: &str) -> Result<()> {
        let template = self.templates.get(template_id)
            .ok_or_else(|| anyhow!("Template not found"))?
            .clone();

        let project = self.projects.get_mut(project_id)
            .ok_or_else(|| anyhow!("Project not found"))?;

        project.endpoints.extend(template.endpoints);
        project.schemas.extend(template.schemas);
        project.updated_at = chrono::Utc::now().to_rfc3339();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_project() {
        let mut builder = APIBuilder::new();
        let project_id = builder.create_project(
            "Test API".to_string(),
            "Test description".to_string()
        ).unwrap();
        
        assert!(builder.get_project(&project_id).is_some());
    }

    #[test]
    fn test_generate_express_code() {
        let mut builder = APIBuilder::new();
        let project_id = builder.create_project(
            "Test API".to_string(),
            "Test description".to_string()
        ).unwrap();

        let endpoint = APIEndpoint {
            id: Uuid::new_v4().to_string(),
            path: "/test".to_string(),
            method: HTTPMethod::GET,
            summary: "Test endpoint".to_string(),
            description: "Test description".to_string(),
            parameters: vec![],
            request_body: None,
            responses: HashMap::new(),
            tags: vec![],
        };

        builder.add_endpoint(&project_id, endpoint).unwrap();
        let code = builder.generate_express_code(&project_id).unwrap();
        
        assert!(code.files.contains_key("package.json"));
        assert!(code.files.contains_key("src/app.js"));
    }
}

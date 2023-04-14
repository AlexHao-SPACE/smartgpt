use std::{error::Error, backtrace::Backtrace, collections::HashMap, fmt::Display};
use async_trait::async_trait;
use reqwest::Client;

mod types;

use serde::{Serialize, Deserialize};
use serde_json::Value;
pub use types::*;

use crate::{Plugin, Command, CommandContext, CommandImpl, EmptyCycle, invoke, BrowseRequest, PluginData, PluginDataNoInvoke, PluginCycle, LLMResponse};

#[derive(Debug, Clone)]
pub struct GoogleNoQueryError;

impl Display for GoogleNoQueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", "'google' command did not receive a query.")
    }
}

impl Error for GoogleNoQueryError {}

pub async fn google(ctx: &mut CommandContext, args: HashMap<String, String>) -> Result<String, Box<dyn Error>> {
    let wolfram_info = ctx.plugin_data.get_data("Google")?;

    let api_key = invoke::<String>(wolfram_info, "get api key", true).await?;
    let api_key: &str = &api_key;
    
    let cse_id = invoke::<String>(wolfram_info, "get cse id", true).await?;
    let cse_id: &str = &cse_id;

    let query = args.get("query").ok_or(GoogleNoQueryError)?;

    let params = [
        ("key", api_key),
        ("cx", cse_id),
        ("q", query),
        ("num", "7")
    ];
    
    let browse_info = ctx.plugin_data.get_data("Browse")?;
    let body = invoke::<String>(browse_info, "browse", BrowseRequest {
        url: "https://www.googleapis.com/customsearch/v1".to_string(),
        params: params.iter()
            .map(|el| (el.0.to_string(), el.1.to_string()))
            .collect::<Vec<_>>()
    }).await?;

    // The conversion to JSON and from JSON is to get rid of unnecessary properties.
    let json_result: Result<SearchResponse, serde_json::Error> = serde_json::from_str(&body);
    let json = match json_result {
        Ok(json) => {
            json
        }
        Err(err) => {
            println!("{:?}", err);
            println!("{}", body);
            return Ok(format!("Unable to parse your Google request for \"{query}\" Try modifying your query or waiting a bit."));
        }
    };
    let text: String = serde_json::to_string(&json)?;
    
    let text = format!(
"{text}

You may want to consider using 'browse-article' to browse the searched websites."
    );

    Ok(text)
}

pub struct GoogleImpl;

#[async_trait]
impl CommandImpl for GoogleImpl {
    async fn invoke(&self, ctx: &mut CommandContext, args: HashMap<String, String>) -> Result<String, Box<dyn Error>> {
        google(ctx, args).await
    }
}

#[derive(Serialize, Deserialize)]
pub struct GoogleData {
    #[serde(rename = "cse id")] pub cse_id: String,
    #[serde(rename = "api key")] pub api_key: String
}

#[async_trait]
impl PluginData for GoogleData {
    async fn apply(&mut self, name: &str, _: Value) -> Result<Value, Box<dyn Error>> {
        match name {
            "get api key" => {
                Ok(self.api_key.clone().into())
            }
            "get cse id" => {
                Ok(self.cse_id.clone().into())
            }
            _ => {
                Err(Box::new(PluginDataNoInvoke("Google".to_string(), name.to_string())))
            }
        }
    }
}

pub struct GoogleCycle;

#[async_trait]
impl PluginCycle for GoogleCycle {
    async fn create_context(&self, context: &mut CommandContext, previous_prompt: Option<&str>) -> Result<Option<String>, Box<dyn Error>> {
        Ok(None)
    }

    async fn apply_removed_response(&self, context: &mut CommandContext, response: &LLMResponse, cmd_output: &str, previous_response: bool) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    async fn create_data(&self, value: Value) -> Option<Box<dyn PluginData>> {
        let data: GoogleData = serde_json::from_value(value).ok()?;
        Some(Box::new(data))
    }
}

pub fn create_google() -> Plugin {
    Plugin {
        name: "Google".to_string(),
        dependencies: vec![ "Browse".to_string() ],
        cycle: Box::new(GoogleCycle),
        commands: vec![
            Command {
                name: "google".to_string(),
                purpose: "Google Search".to_string(),
                args: vec![
                    ("query".to_string(), "The request to search. Create a short, direct query with keywords.".to_string())
                ],
                run: Box::new(GoogleImpl)
            }
        ]
    }
}
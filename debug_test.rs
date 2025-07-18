use ai_proxy::providers::anthropic::*;

fn main() {
    let long_content = "a".repeat(200001);
    let request = AnthropicRequest {
        model: "claude-3-sonnet-20240229".to_string(),
        messages: vec![
            Message {
                role: "user".to_string(),
                content: long_content,
            }
        ],
        max_tokens: 1000,
        temperature: None,
        top_p: None,
        stream: None,
    };
    let result = request.validate();
    match result {
        Ok(_) => println!("Validation passed"),
        Err(e) => println!("Error: {}", e),
    }
}

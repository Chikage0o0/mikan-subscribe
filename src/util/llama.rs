use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use super::reqwest::client;

static LLAMA: OnceLock<Llama> = OnceLock::new();

pub struct Llama {
    model: String,
    url: String,
    token: String,
}

impl Llama {
    pub fn get() -> Option<&'static Llama> {
        LLAMA.get()
    }

    pub fn init(model: &str, url: &str, token: &str) -> Result<(), String> {
        let llama = Llama {
            model: model.to_string(),
            url: url.to_string(),
            token: token.to_string(),
        };
        LLAMA
            .set(llama)
            .map_err(|_| "Llama has been initialized".to_string())?;

        Ok(())
    }

    pub async fn decode(&self, filename: &str) -> Result<ContentResponse, String> {
        let messages = vec![
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::System,
                content: Some(SYSTEM.to_string()),
            },
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::User,
                content: Some(
                    "[Billion Meta Lab] 恋语轻唱 Sasayaku You ni Koi wo Utau [02][1080][CHS].mp4"
                        .to_string(),
                ),
            },
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::Assistant,
                content: Some(r#"The title in the filename is "恋语轻唱 Sasayaku You ni Koi wo Utau", but "恋语轻唱" has the same meaning as the Japanese Romaji "Sasayaku You ni Koi wo Utau". In this case, it is preferred to use the alphabetic title.There is no clear season identifier, which usually defaults to Season 1. Since "[02]" in the filename indicates this is the 2nd episode.
{"title": "Sasayaku You ni Koi wo Utau", "season": 1, "episode": 2}"#.to_string()),
            },
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::User,
                content: Some("[Up to 21°C] 永生 第三季 - 06 (B-Global Donghua 1920x1080 HEVC AAC MKV) [9F7CAB79].mkv".to_string()),
            },
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::Assistant,
                content: Some(r#"The title in the filename is "永生" The explicit designation of "第三季" indicates that this is Season 3, while "06" points out that this is Episode 6.
{"title": "永生", "season": 3, "episode": 6}"#.to_string()),
            },
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::User,
                content: Some("[GJ.Y] 我内心危险的东西2 - 05 (B-Global 1920x1080 HEVC AAC MKV) [60D8C635].mkv".to_string()),
            },
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::Assistant,
                content: Some(r#"The title in the filename is "我内心危险的东西". The number "2" following the title indicates that this is Season 2, and "05" indicates that this is Episode 5.
{"title": "我内心危险的东西", "season": 2, "episode": 5}"#.to_string()),
            },
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::User,
                content: Some("[Up to 21°C] Shinigami Bocchan to Kuro Maid 3rd Season - 30 (CR 1920x1080 AVC AAC MKV) [CE516DAA].mkv".to_string()),
            },
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::Assistant,
                content: Some(r#"The title in the filename is "Shinigami Bocchan to Kuro Maid". The explicit designation of "3rd Season" indicates that this is Season 3, while "30" points out that this is Episode 30.
{"title": "Shinigami Bocchan to Kuro Maid", "season": 3, "episode": 30}"#.to_string()),
            },
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::User,
                content: Some("[Sakurato] Dungeon Meshi [17v2][AVC-8bit 1080p AAC][CHT].mp4".to_string()),
            },
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::Assistant,
                content: Some(r#"The title in the filename is "Dungeon Meshi". There is no explicit season identifier, which usually defaults to Season 1. The "17v2" indicates that this is Episode 17, with "v2" likely indicating a version or revision.
{"title": "Dungeon Meshi", "season": 1, "episode": 17}"#.to_string()),
            },
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::User,
                content: Some("[Up to 21°C] Mushoku Tensei II -  Isekai Ittara Honki Dasu Part 2 - 18 (ABEMA 1920x1080 AVC AAC MP4) [05628308].mp4".to_string()),
            },
            ChatCompletionMessage{
                role: ChatCompletionMessageRole::Assistant,
                content: Some(r#"The title in the filename is "Mushoku Tensei II - Isekai Ittara Honki Dasu Part 2". The "II" indicates that this is the second season, and "18" indicates that this is Episode 18.
{"title": "Mushoku Tensei II - Isekai Ittara Honki Dasu Part 2", "season": 2, "episode": 18}"#.to_string()),
            },
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::User,
                content: Some(filename.to_string()),
            },
        ];

        let chat_req = ChatRequest {
            model: self.model.clone(),
            messages,
            temperature: 0.01,
        };

        let res = client()
            .post(&self.url)
            .bearer_auth(&self.token)
            .json(&chat_req)
            .send()
            .await
            .map_err(|e| format!("Request error: {}", e))?;

        let res = res
            .json::<Response>()
            .await
            .map_err(|e| format!("Json error: {}", e))?;

        let choice = res.choices.first().ok_or("No choice")?;
        let message = &choice.message.content;
        for line in message.lines() {
            // println!("{}", line);
            if let Ok(content) = serde_json::from_str::<ContentResponse>(line) {
                return Ok(content);
            }
        }

        Err("No content".to_string())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentResponse {
    title: String,
    season: u32,
    pub episode: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MessageResponse {
    role: ChatCompletionMessageRole,
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChoiceResponse {
    index: usize,
    message: MessageResponse,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Response {
    choices: Vec<ChoiceResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatCompletionMessage>,
    temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatCompletionMessage {
    role: ChatCompletionMessageRole,
    content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ChatCompletionMessageRole {
    System,
    User,
    Assistant,
}

const SYSTEM: &str = r#"Please parse the following animation file names into JSON objects containing the keys "title," "season," and "episode." Ensure the accuracy of your parsing results."#;

#[cfg(test)]
mod tests {
    use crate::util::{self, reqwest::init_client};

    use super::*;

    #[tokio::test]
    async fn test_decode() {
        let settings = util::config::Settings::load_from_file("settings.json").unwrap();
        let _ = init_client(settings.proxy).unwrap();
        if let Some(llama) = settings.llama {
            Llama::init(&llama.model, &llama.url, &llama.token).unwrap();
        }

        let filename = "[ANi] 模擬後宮體驗 - 01 [1080P][Baha][WEB-DL][AAC AVC][CHT].mp4";
        let content = Llama::get().unwrap().decode(filename).await.unwrap();
        // assert_eq!(content.title, "Shuumatsu Train Doko e Iku");
        // assert_eq!(content.season, 1);
        // assert_eq!(content.episode, 11);
        dbg!(content);
    }
}

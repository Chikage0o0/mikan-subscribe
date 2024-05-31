use std::{path::Path, sync::OnceLock};

use llama_cpp::{
    standard_sampler::{SamplerStage, StandardSampler},
    LlamaModel, LlamaParams, SessionParams,
};
use serde::{Deserialize, Serialize};

static LLAMA: OnceLock<Llama> = OnceLock::new();

pub struct Llama {
    model: LlamaModel,
    session_params: SessionParams,
}

impl Llama {
    pub fn get() -> Option<&'static Llama> {
        LLAMA.get()
    }

    pub fn init(model_path: &Path) -> Result<(), String> {
        if LLAMA.get().is_some() {
            return Ok(());
        }

        let model =
            LlamaModel::load_from_file(model_path.to_str().unwrap(), LlamaParams::default())
                .map_err(|e| e.to_string())?;
        let session_params = SessionParams::default();
        let llama = Llama {
            model,
            session_params,
        };
        let _ = LLAMA.set(llama);
        Ok(())
    }

    pub fn decode(&self, filename: &str) -> Result<Response, String> {
        let now = std::time::SystemTime::now();
        let mut ctx = self
            .model
            .create_session(self.session_params.clone())
            .map_err(|e| e.to_string())?;
        println!("{:?}", now.elapsed().unwrap());
        ctx.advance_context(format!(
            "<bos><start_of_turn>user\n{}\n{}<end_of_turn>\n<start_of_turn>model\n",
            r#"You are a json api service.I will give you a anime file name.You need to return a Json like this: {"title":string,"season":number,"episode":number}"#,
            filename
        ))
        .unwrap();

        let sampler = StandardSampler::new_softmax(
            vec![
                // SamplerStage::RepetitionPenalty {
                //     repetition_penalty: 1.1,
                //     frequency_penalty: 0.0,
                //     presence_penalty: 0.0,
                //     last_n: 16,
                // },
                SamplerStage::Temperature(0.01),
            ],
            1,
        );
        let completions = ctx
            .start_completing_with(sampler, 1024)
            .unwrap()
            .into_strings();

        let mut output = String::new();

        for completion in completions {
            if matches!(completion.as_str(), "<start_of_turn>" | "<end_of_turn>") {
                break;
            }

            output.push_str(&completion);
        }

        let response = serde_json::from_str(&output).map_err(|e| e.to_string())?;

        Ok(response)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    title: String,
    season: u32,
    pub episode: u32,
}

#[test]
fn ss() {
    let now = std::time::SystemTime::now();
    let _ = Llama::init(Path::new("gemma2b.gguf"));
    println!("{:?}", now.elapsed().unwrap());

    let response = Llama::get()
        .unwrap()
        .decode("[Nekomoe kissaten][Seiyuu Radio no Uraomote][08][1080p][JPTC].mp4");

    println!("{:?}", response);
}

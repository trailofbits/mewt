//! Small, workflow-independent client for TypeSafe System One.
//!
//! Callers own the state, rubrics, and interpretation of answers. In particular,
//! a Score is an ordinal rubric position, not a measured TCAP probability.

use std::collections::BTreeMap;
use std::time::Duration;

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

const BASE_URL: &str = "https://api.typesafe.ai";

/// A question's instructions and criteria may be a string or structured JSON.
/// `state` must be a string, object, or array as required by the API.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Question {
    Noul {
        instructions: Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        criteria: Option<NoulCriteria>,
    },
    Choice {
        instructions: Value,
        criteria: BTreeMap<String, Value>,
    },
    Score {
        instructions: Value,
        criteria: Vec<Value>,
    },
}

#[derive(Clone, Debug, Serialize)]
pub struct NoulCriteria {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#true: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#false: Option<Value>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Evaluation {
    pub state: Value,
    pub model: String,
    pub questions: BTreeMap<String, Question>,
}

impl Evaluation {
    pub fn new(state: Value, questions: BTreeMap<String, Question>) -> Self {
        Self {
            state,
            model: "jev-latest".into(),
            questions,
        }
    }

    fn validate(&self) -> Result<(), Error> {
        if !matches!(
            self.state,
            Value::String(_) | Value::Object(_) | Value::Array(_)
        ) {
            return Err(Error::InvalidRequest(
                "state must be a string, object, or array",
            ));
        }
        if self.model.trim().is_empty() || self.questions.is_empty() {
            return Err(Error::InvalidRequest(
                "model and questions must not be empty",
            ));
        }
        for (id, question) in &self.questions {
            if id.is_empty() {
                return Err(Error::InvalidRequest("question IDs must not be empty"));
            }
            match question {
                Question::Score { criteria, .. } if !(2..=10).contains(&criteria.len()) => {
                    return Err(Error::InvalidRequest("score requires 2 to 10 levels"));
                }
                Question::Choice { criteria, .. }
                    if criteria.is_empty() || criteria.len() > 255 =>
                {
                    return Err(Error::InvalidRequest("choice requires 1 to 255 options"));
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Answer {
    Noul {
        noul: f64,
    },
    Choice {
        choice: String,
        confidence: f64,
        probabilities: BTreeMap<String, f64>,
    },
    Score {
        score: f64,
        confidence: f64,
        legend: BTreeMap<String, Value>,
        probabilities: BTreeMap<String, f64>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EvaluationResult {
    pub model: String,
    pub answers: BTreeMap<String, Answer>,
    pub usage: Usage,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("missing TypeSafe API key (set TYPESAFE_API_KEY)")]
    MissingApiKey,
    #[error("invalid TypeSafe API key format")]
    InvalidApiKey,
    #[error("invalid TypeSafe request: {0}")]
    InvalidRequest(&'static str),
    #[error("TypeSafe transport error: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("TypeSafe API returned HTTP {0}")]
    Http(StatusCode),
    #[error("invalid TypeSafe response: {0}")]
    InvalidResponse(String),
}

/// Reusable async client. Does not log secrets, request bodies, or response bodies.
/// Retries 429/529 twice with bounded exponential backoff; callers can retry
/// other failures or choose to fall back to ordinary Mewt results.
#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl Client {
    pub fn from_env() -> Result<Self, Error> {
        let key = std::env::var("TYPESAFE_API_KEY").map_err(|_| Error::MissingApiKey)?;
        Self::new(key)
    }

    pub fn new(api_key: impl Into<String>) -> Result<Self, Error> {
        let key = api_key.into();
        let key = key.trim();
        if key.is_empty() {
            return Err(Error::MissingApiKey);
        }
        if !key.is_ascii() || !key.bytes().all(|b| b.is_ascii_graphic()) {
            return Err(Error::InvalidApiKey);
        }
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()?;
        Ok(Self {
            http,
            api_key: key.into(),
            base_url: BASE_URL.into(),
        })
    }

    /// Override the API root (e.g. a local test server). Do not use untrusted URLs
    /// with a real key: the bearer credential is sent to this endpoint.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_owned();
        self
    }

    pub async fn evaluate(&self, request: &Evaluation) -> Result<EvaluationResult, Error> {
        request.validate()?;
        for attempt in 0..=2 {
            let response = self
                .http
                .post(format!("{}/v1/systemone", self.base_url))
                .bearer_auth(&self.api_key)
                .json(request)
                .send()
                .await?;
            let status = response.status();
            if (status == StatusCode::TOO_MANY_REQUESTS || status.as_u16() == 529) && attempt < 2 {
                let delay = response
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .map(|s| s.min(5))
                    .unwrap_or(1 << attempt);
                tokio::time::sleep(Duration::from_secs(delay)).await;
                continue;
            }
            if !status.is_success() {
                return Err(Error::Http(status));
            }
            let result: EvaluationResult = response
                .json()
                .await
                .map_err(|_| Error::InvalidResponse("malformed response JSON".into()))?;
            validate_answers(request, &result)?;
            return Ok(result);
        }
        unreachable!("retry loop returns on last attempt")
    }
}

pub(crate) fn validate_answers(
    request: &Evaluation,
    result: &EvaluationResult,
) -> Result<(), Error> {
    if result.answers.len() != request.questions.len() || result.model.trim().is_empty() {
        return Err(Error::InvalidResponse(
            "missing or unexpected answers/model".into(),
        ));
    }
    for (id, question) in &request.questions {
        let answer = result
            .answers
            .get(id)
            .ok_or_else(|| Error::InvalidResponse(format!("missing answer for {id}")))?;
        let valid = match (question, answer) {
            (Question::Noul { .. }, Answer::Noul { noul }) => probability(*noul),
            (
                Question::Choice { criteria, .. },
                Answer::Choice {
                    choice,
                    confidence,
                    probabilities,
                },
            ) => {
                probability(*confidence)
                    && criteria.contains_key(choice)
                    && probabilities.keys().eq(criteria.keys())
                    && distribution(probabilities)
            }
            (
                Question::Score { criteria, .. },
                Answer::Score {
                    score,
                    confidence,
                    legend,
                    probabilities,
                },
            ) => {
                probability(*confidence)
                    && score.is_finite()
                    && *score >= 0.0
                    && *score <= (criteria.len() - 1) as f64
                    && legend.len() == criteria.len()
                    && criteria
                        .iter()
                        .enumerate()
                        .all(|(i, level)| legend.get(&i.to_string()) == Some(level))
                    && probabilities.len() == criteria.len()
                    && probabilities.keys().eq((0..criteria.len())
                        .map(|i| i.to_string())
                        .collect::<Vec<_>>()
                        .iter())
                    && distribution(probabilities)
                    && (probabilities
                        .iter()
                        .filter_map(|(level, p)| level.parse::<usize>().ok().map(|i| i as f64 * p))
                        .sum::<f64>()
                        - score)
                        .abs()
                        < 0.05
            }
            _ => false,
        };
        if !valid {
            return Err(Error::InvalidResponse(format!("invalid answer for {id}")));
        }
    }
    Ok(())
}

fn probability(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn distribution(values: &BTreeMap<String, f64>) -> bool {
    values.values().all(|v| probability(*v)) && (values.values().sum::<f64>() - 1.0).abs() < 0.02
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn round_trip_questions_and_answers() {
        let request = Evaluation::new(
            json!({"mutant": "x + 1"}),
            BTreeMap::from([
                (
                    "priority".into(),
                    Question::Score {
                        instructions: json!("How useful?"),
                        criteria: vec![json!("low"), json!("high")],
                    },
                ),
                (
                    "kind".into(),
                    Question::Choice {
                        instructions: json!("Which kind?"),
                        criteria: BTreeMap::from([("other".into(), json!(null))]),
                    },
                ),
                (
                    "reachable".into(),
                    Question::Noul {
                        instructions: json!("Is it reachable?"),
                        criteria: None,
                    },
                ),
            ]),
        );
        assert_eq!(
            serde_json::to_value(&request).unwrap()["questions"]["priority"]["type"],
            "score"
        );
        let result: EvaluationResult = serde_json::from_value(json!({
            "model":"jev-1", "usage":{"input_tokens":12,"output_tokens":3},
            "answers": {
                "priority":{"type":"score","score":0.75,"confidence":0.6,"legend":{"0":"low","1":"high"},"probabilities":{"0":0.25,"1":0.75}},
                "kind":{"type":"choice","choice":"other","confidence":1.0,"probabilities":{"other":1.0}},
                "reachable":{"type":"noul","noul":0.4}
            }
        })).unwrap();
        validate_answers(&request, &result).unwrap();
    }

    #[tokio::test]
    async fn posts_to_systemone_and_reports_http_failures() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            for status in ["200 OK", "401 Unauthorized"] {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut bytes = vec![0_u8; 8192];
                let mut received = Vec::new();
                loop {
                    let count = socket.read(&mut bytes).await.unwrap();
                    assert!(count > 0);
                    received.extend_from_slice(&bytes[..count]);
                    if received.windows(4).any(|chunk| chunk == b"\r\n\r\n") {
                        break;
                    }
                }
                let headers = String::from_utf8_lossy(&received);
                assert!(headers.starts_with("POST /v1/systemone HTTP/1.1"));
                assert!(
                    headers
                        .to_ascii_lowercase()
                        .contains("authorization: bearer test-key")
                );
                let body = r#"{"model":"jev-1","answers":{"q":{"type":"noul","noul":0.9}},"usage":{"input_tokens":10,"output_tokens":2}}"#;
                let reply = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                socket.write_all(reply.as_bytes()).await.unwrap();
            }
        });
        let request = Evaluation::new(
            json!("source"),
            BTreeMap::from([(
                "q".into(),
                Question::Noul {
                    instructions: json!("Is it useful?"),
                    criteria: None,
                },
            )]),
        );
        let client = Client::new("test-key").unwrap().with_base_url(url);
        let result = client.evaluate(&request).await.unwrap();
        assert_eq!(result.usage.input_tokens, 10);
        assert!(matches!(
            client.evaluate(&request).await,
            Err(Error::Http(StatusCode::UNAUTHORIZED))
        ));
        server.await.unwrap();
    }

    #[test]
    fn rejects_bad_requests_and_answers() {
        let mut request = Evaluation::new(json!(null), BTreeMap::new());
        assert!(request.validate().is_err());
        request.state = json!("code");
        request.questions.insert(
            "q".into(),
            Question::Score {
                instructions: json!("priority"),
                criteria: vec![json!("only one")],
            },
        );
        assert!(request.validate().is_err());
        request.questions.insert(
            "q".into(),
            Question::Noul {
                instructions: json!("yes?"),
                criteria: None,
            },
        );
        let result: EvaluationResult = serde_json::from_value(json!({"model":"jev-1","usage":{"input_tokens":1,"output_tokens":1},"answers":{"q":{"type":"noul","noul":1.5}}})).unwrap();
        assert!(validate_answers(&request, &result).is_err());
        let wrong: EvaluationResult = serde_json::from_value(json!({"model":"jev-1","usage":{"input_tokens":1,"output_tokens":1},"answers":{"q":{"type":"choice","choice":"a","confidence":1.0,"probabilities":{"a":1.0}}}})).unwrap();
        assert!(validate_answers(&request, &wrong).is_err());
    }
}

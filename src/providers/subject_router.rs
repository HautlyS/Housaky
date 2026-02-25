use super::traits::{ChatMessage, ChatRequest, ChatResponse};
use super::Provider;
use async_trait::async_trait;
use std::collections::HashMap;

fn is_limit_saturated(err: &anyhow::Error) -> bool {
    err.to_string()
        .starts_with(crate::providers::limit::LimitProvider::SATURATED_ERROR_PREFIX)
}

/// A subject-aware provider router.
///
/// If the `model` argument is of the form `hint:<subject>`, this router will:
/// - look up subject routing config (provider_chain + model)
/// - attempt providers in order
/// - (limits are enforced by wrapping providers at construction time)
///
/// Otherwise, it falls back to the default provider.
pub struct SubjectRouterProvider {
    providers: HashMap<String, Box<dyn Provider>>, // provider_name -> provider
    default_provider: String,
    default_model: String,
    subjects: HashMap<String, SubjectRoute>,
}

#[derive(Clone, Debug)]
pub struct SubjectRoute {
    pub provider_chain: Vec<String>,
    pub model: String,
    pub temperature: Option<f64>,
    pub parallel_limits: HashMap<String, usize>,
}

impl SubjectRouterProvider {
    pub fn new(
        providers: HashMap<String, Box<dyn Provider>>,
        default_provider: String,
        default_model: String,
        subjects: HashMap<String, SubjectRoute>,
    ) -> Self {
        Self {
            providers,
            default_provider,
            default_model,
            subjects,
        }
    }

    fn resolve(&self, model: &str) -> ResolvedRoute {
        if let Some(subject) = model.strip_prefix("hint:") {
            if let Some(route) = self.subjects.get(subject) {
                return ResolvedRoute {
                    provider_chain: route.provider_chain.clone(),
                    model: route.model.clone(),
                    temperature: route.temperature,
                };
            }
        }

        ResolvedRoute {
            provider_chain: vec![self.default_provider.clone()],
            model: if model.starts_with("hint:") {
                self.default_model.clone()
            } else {
                model.to_string()
            },
            temperature: None,
        }
    }
}

#[derive(Clone, Debug)]
struct ResolvedRoute {
    provider_chain: Vec<String>,
    model: String,
    temperature: Option<f64>,
}

#[async_trait]
impl Provider for SubjectRouterProvider {
    async fn chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        let route = self.resolve(model);
        let temp = route.temperature.unwrap_or(temperature);

        let mut errors = Vec::new();
        for (idx, provider_name) in route.provider_chain.iter().enumerate() {
            let Some(provider) = self.providers.get(provider_name) else {
                errors.push(format!("{provider_name}: provider not available"));
                continue;
            };

            tracing::info!(
                route = model,
                resolved_model = route.model.as_str(),
                provider = provider_name.as_str(),
                chain_index = idx,
                "provider_delegate_attempt"
            );

            match provider
                .chat_with_system(system_prompt, message, &route.model, temp)
                .await
            {
                Ok(resp) => {
                    tracing::info!(
                        route = model,
                        resolved_model = route.model.as_str(),
                        provider = provider_name.as_str(),
                        chain_index = idx,
                        "provider_delegate_success"
                    );
                    return Ok(resp);
                }
                Err(e) => {
                    if is_limit_saturated(&e) {
                        tracing::warn!(
                            route = model,
                            resolved_model = route.model.as_str(),
                            provider = provider_name.as_str(),
                            chain_index = idx,
                            error = %e,
                            "provider_delegate_saturated_failover"
                        );
                        errors.push(format!("{provider_name}: saturated"));
                        continue;
                    }

                    tracing::warn!(
                        route = model,
                        resolved_model = route.model.as_str(),
                        provider = provider_name.as_str(),
                        chain_index = idx,
                        error = %e,
                        "provider_delegate_error_failover"
                    );
                    errors.push(format!("{provider_name}: {e}"));
                }
            }
        }

        Err(anyhow::anyhow!(
            "All providers failed for route {}: {}",
            model,
            errors.join(" | ")
        ))
    }

    async fn chat_with_history(
        &self,
        messages: &[ChatMessage],
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        let route = self.resolve(model);
        let temp = route.temperature.unwrap_or(temperature);

        let mut errors = Vec::new();
        for (idx, provider_name) in route.provider_chain.iter().enumerate() {
            let Some(provider) = self.providers.get(provider_name) else {
                errors.push(format!("{provider_name}: provider not available"));
                continue;
            };

            tracing::info!(
                route = model,
                resolved_model = route.model.as_str(),
                provider = provider_name.as_str(),
                chain_index = idx,
                "provider_delegate_attempt"
            );

            match provider.chat_with_history(messages, &route.model, temp).await {
                Ok(resp) => {
                    tracing::info!(
                        route = model,
                        resolved_model = route.model.as_str(),
                        provider = provider_name.as_str(),
                        chain_index = idx,
                        "provider_delegate_success"
                    );
                    return Ok(resp);
                }
                Err(e) => {
                    if is_limit_saturated(&e) {
                        tracing::warn!(
                            route = model,
                            resolved_model = route.model.as_str(),
                            provider = provider_name.as_str(),
                            chain_index = idx,
                            error = %e,
                            "provider_delegate_saturated_failover"
                        );
                        errors.push(format!("{provider_name}: saturated"));
                        continue;
                    }

                    tracing::warn!(
                        route = model,
                        resolved_model = route.model.as_str(),
                        provider = provider_name.as_str(),
                        chain_index = idx,
                        error = %e,
                        "provider_delegate_error_failover"
                    );
                    errors.push(format!("{provider_name}: {e}"));
                }
            }
        }

        Err(anyhow::anyhow!(
            "All providers failed for route {}: {}",
            model,
            errors.join(" | ")
        ))
    }

    async fn chat(
        &self,
        request: ChatRequest<'_>,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        let route = self.resolve(model);
        let temp = route.temperature.unwrap_or(temperature);

        let mut errors = Vec::new();
        for (idx, provider_name) in route.provider_chain.iter().enumerate() {
            let Some(provider) = self.providers.get(provider_name) else {
                errors.push(format!("{provider_name}: provider not available"));
                continue;
            };

            tracing::info!(
                route = model,
                resolved_model = route.model.as_str(),
                provider = provider_name.as_str(),
                chain_index = idx,
                "provider_delegate_attempt"
            );

            match provider.chat(request, &route.model, temp).await {
                Ok(resp) => {
                    tracing::info!(
                        route = model,
                        resolved_model = route.model.as_str(),
                        provider = provider_name.as_str(),
                        chain_index = idx,
                        "provider_delegate_success"
                    );
                    return Ok(resp);
                }
                Err(e) => {
                    if is_limit_saturated(&e) {
                        tracing::warn!(
                            route = model,
                            resolved_model = route.model.as_str(),
                            provider = provider_name.as_str(),
                            chain_index = idx,
                            error = %e,
                            "provider_delegate_saturated_failover"
                        );
                        errors.push(format!("{provider_name}: saturated"));
                        continue;
                    }

                    tracing::warn!(
                        route = model,
                        resolved_model = route.model.as_str(),
                        provider = provider_name.as_str(),
                        chain_index = idx,
                        error = %e,
                        "provider_delegate_error_failover"
                    );
                    errors.push(format!("{provider_name}: {e}"));
                }
            }
        }

        Err(anyhow::anyhow!(
            "All providers failed for route {}: {}",
            model,
            errors.join(" | ")
        ))
    }

    fn supports_native_tools(&self) -> bool {
        self.providers
            .get(&self.default_provider)
            .map(|p| p.supports_native_tools())
            .unwrap_or(false)
    }

    async fn warmup(&self) -> anyhow::Result<()> {
        for (name, provider) in &self.providers {
            tracing::info!(provider = name, "Warming up subject-routed provider");
            if let Err(e) = provider.warmup().await {
                tracing::warn!(provider = name, "Warmup failed (non-fatal): {e}");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FailProvider;

    #[async_trait]
    impl Provider for FailProvider {
        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            anyhow::bail!("fail")
        }
    }

    struct OkProvider(&'static str);

    #[async_trait]
    impl Provider for OkProvider {
        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            Ok(self.0.to_string())
        }
    }

    #[tokio::test]
    async fn hint_subject_tries_chain_until_success() {
        let mut providers: HashMap<String, Box<dyn Provider>> = HashMap::new();
        providers.insert("p1".to_string(), Box::new(FailProvider));
        providers.insert("p2".to_string(), Box::new(OkProvider("ok")));

        let mut subjects = HashMap::new();
        subjects.insert(
            "code".to_string(),
            SubjectRoute {
                provider_chain: vec!["p1".to_string(), "p2".to_string()],
                model: "m".to_string(),
                temperature: None,
                parallel_limits: HashMap::new(),
            },
        );

        let router = SubjectRouterProvider::new(
            providers,
            "p1".to_string(),
            "default".to_string(),
            subjects,
        );

        let out = router
            .chat_with_system(None, "hi", "hint:code", 0.7)
            .await
            .unwrap();
        assert_eq!(out, "ok");
    }
}

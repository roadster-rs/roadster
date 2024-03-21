use crate::app_context::AppContext;
use crate::initializer::normalize_path::NormalizePathConfig;
use serde_derive::{Deserialize, Serialize};

pub const PRIORITY_FIRST: i32 = -10_000;
pub const PRIORITY_LAST: i32 = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct Initializer {
    pub default_enable: bool,
    pub normalize_path: InitializerConfig<NormalizePathConfig>,
}

impl Default for Initializer {
    fn default() -> Self {
        let normalize_path: InitializerConfig<NormalizePathConfig> = Default::default();
        let normalize_path = normalize_path.set_priority(PRIORITY_LAST);

        Self {
            default_enable: true,
            normalize_path,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct CommonConfig {
    // Optional so we can tell the difference between a consumer explicitly enabling/disabling
    // the initializer, vs the initializer being enabled/disabled by default.
    // If this is `None`, the value will match the value of `Initializer#default_enable`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,
    pub priority: i32,
}

impl CommonConfig {
    pub fn set_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn enabled(&self, context: &AppContext) -> bool {
        self.enable
            .unwrap_or(context.config.initializer.default_enable)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct InitializerConfig<T: Default> {
    #[serde(flatten)]
    pub common: CommonConfig,
    #[serde(flatten)]
    pub custom: T,
}

impl<T: Default> InitializerConfig<T> {
    pub fn set_priority(mut self, priority: i32) -> Self {
        self.common = self.common.set_priority(priority);
        self
    }
}

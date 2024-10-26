use crate::lang::{CustomLang, LanguageGlobs, SerializableInjection, SgLang};
use crate::utils::{ErrorContext as EC, RuleOverwrite, RuleTrace};

use anyhow::{Context, Result};
use ast_grep_config::{
  from_str, from_yaml_string, DeserializeEnv, GlobalRules, RuleCollection, RuleConfig,
};
use ast_grep_language::config_file_type;
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TestConfig {
  pub test_dir: PathBuf,
  /// Specify the directory containing snapshots. The path is relative to `test_dir`
  #[serde(skip_serializing_if = "Option::is_none")]
  pub snapshot_dir: Option<PathBuf>,
}

impl From<PathBuf> for TestConfig {
  fn from(path: PathBuf) -> Self {
    TestConfig {
      test_dir: path,
      snapshot_dir: None,
    }
  }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AstGrepConfig {
  /// YAML rule directories
  pub rule_dirs: Vec<PathBuf>,
  /// test configurations
  #[serde(skip_serializing_if = "Option::is_none")]
  pub test_configs: Option<Vec<TestConfig>>,
  /// util rules directories
  #[serde(skip_serializing_if = "Option::is_none")]
  pub util_dirs: Option<Vec<PathBuf>>,
  /// configuration for custom languages
  #[serde(skip_serializing_if = "Option::is_none")]
  pub custom_languages: Option<HashMap<String, CustomLang>>,
  /// additional file globs for languages
  #[serde(skip_serializing_if = "Option::is_none")]
  pub language_globs: Option<LanguageGlobs>,
  /// injection config for embedded languages
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub language_injections: Vec<SerializableInjection>,
}

#[derive(Clone)]
pub struct ProjectConfig {
  pub project_dir: PathBuf,
  pub sg_config: AstGrepConfig,
}

impl ProjectConfig {
  pub fn by_config_path_must(config_path: Option<PathBuf>) -> Result<Self> {
    Self::discover_project(config_path, None)?.ok_or_else(|| anyhow::anyhow!(EC::ProjectNotExist))
  }
  pub fn by_config_path(config_path: Option<PathBuf>) -> Result<Option<Self>> {
    Self::discover_project(config_path, None)
  }
  // return None if config file does not exist
  fn discover_project(config_path: Option<PathBuf>, base: Option<&Path>) -> Result<Option<Self>> {
    let config_path =
      find_config_path_with_default(config_path, base).context(EC::ReadConfiguration)?;
    // NOTE: if config file does not exist, return None
    // this is not 100% correct because of racing condition
    if !config_path.is_file() {
      return Ok(None);
    }
    let config_str = read_to_string(&config_path).context(EC::ReadConfiguration)?;
    let sg_config: AstGrepConfig = from_str(&config_str).context(EC::ParseConfiguration)?;
    let project_dir = config_path
      .parent()
      .expect("config file must have parent directory")
      .to_path_buf();
    Ok(Some(ProjectConfig {
      project_dir,
      sg_config,
    }))
  }

  pub fn find_rules(
    &self,
    rule_overwrite: RuleOverwrite,
  ) -> Result<(RuleCollection<SgLang>, RuleTrace)> {
    let global_rules = find_util_rules(self)?;
    read_directory_yaml(self, global_rules, rule_overwrite)
  }
}

pub fn register_custom_language(project_config: Option<ProjectConfig>) -> Result<()> {
  // do not report error if no sgconfig.yml is found
  let Some(project_config) = project_config else {
    return Ok(());
  };
  let ProjectConfig {
    project_dir,
    sg_config,
  } = project_config;
  if let Some(custom_langs) = sg_config.custom_languages {
    SgLang::register_custom_language(project_dir, custom_langs);
  }
  if let Some(globs) = sg_config.language_globs {
    SgLang::register_globs(globs)?;
  }
  SgLang::register_injections(sg_config.language_injections)?;
  Ok(())
}

fn build_util_walker(base_dir: &Path, util_dirs: &Option<Vec<PathBuf>>) -> Option<WalkBuilder> {
  let mut util_dirs = util_dirs.as_ref()?.iter();
  let first = util_dirs.next()?;
  let mut walker = WalkBuilder::new(base_dir.join(first));
  for dir in util_dirs {
    walker.add(base_dir.join(dir));
  }
  Some(walker)
}

fn find_util_rules(config: &ProjectConfig) -> Result<GlobalRules<SgLang>> {
  let ProjectConfig {
    project_dir,
    sg_config,
  } = config;
  let Some(mut walker) = build_util_walker(project_dir, &sg_config.util_dirs) else {
    return Ok(GlobalRules::default());
  };
  let mut utils = vec![];
  let walker = walker.types(config_file_type()).build();
  for dir in walker {
    let config_file = dir.with_context(|| EC::WalkRuleDir(PathBuf::new()))?;
    // file_type is None only if it is stdin, safe to panic here
    if !config_file
      .file_type()
      .expect("file type should be available for non-stdin")
      .is_file()
    {
      continue;
    }
    let path = config_file.path();
    let file = read_to_string(path)?;
    let new_configs = from_str(&file)?;
    utils.push(new_configs);
  }

  let ret = DeserializeEnv::parse_global_utils(utils).context(EC::InvalidGlobalUtils)?;
  Ok(ret)
}

fn read_directory_yaml(
  config: &ProjectConfig,
  global_rules: GlobalRules<SgLang>,
  rule_overwrite: RuleOverwrite,
) -> Result<(RuleCollection<SgLang>, RuleTrace)> {
  let mut configs = vec![];
  let ProjectConfig {
    project_dir,
    sg_config,
  } = config;
  for dir in &sg_config.rule_dirs {
    let dir_path = project_dir.join(dir);
    let walker = WalkBuilder::new(&dir_path)
      .types(config_file_type())
      .build();
    for dir in walker {
      let config_file = dir.with_context(|| EC::WalkRuleDir(dir_path.clone()))?;
      // file_type is None only if it is stdin, safe to panic here
      if !config_file
        .file_type()
        .expect("file type should be available for non-stdin")
        .is_file()
      {
        continue;
      }
      let path = config_file.path();
      let new_configs = read_rule_file(path, Some(&global_rules))?;
      configs.extend(new_configs);
    }
  }
  let total_rule_count = configs.len();

  let configs = rule_overwrite.process_configs(configs)?;
  let collection = RuleCollection::try_new(configs).context(EC::GlobPattern)?;
  let effective_rule_count = collection.total_rule_count();
  let trace = RuleTrace {
    effective_rule_count,
    skipped_rule_count: total_rule_count - effective_rule_count,
  };
  Ok((collection, trace))
}

pub fn read_rule_file(
  path: &Path,
  global_rules: Option<&GlobalRules<SgLang>>,
) -> Result<Vec<RuleConfig<SgLang>>> {
  let yaml = read_to_string(path).with_context(|| EC::ReadRule(path.to_path_buf()))?;
  let parsed = if let Some(globals) = global_rules {
    from_yaml_string(&yaml, globals)
  } else {
    from_yaml_string(&yaml, &Default::default())
  };
  parsed.with_context(|| EC::ParseRule(path.to_path_buf()))
}

const CONFIG_FILE: &str = "sgconfig.yml";

fn find_config_path_with_default(
  config_path: Option<PathBuf>,
  base: Option<&Path>,
) -> Result<PathBuf> {
  if let Some(config) = config_path {
    return Ok(config);
  }
  let mut path = if let Some(base) = base {
    base.to_path_buf()
  } else {
    std::env::current_dir()?
  };
  loop {
    let maybe_config = path.join(CONFIG_FILE);
    if maybe_config.exists() {
      break Ok(maybe_config);
    }
    if let Some(parent) = path.parent() {
      path = parent.to_path_buf();
    } else {
      break Ok(PathBuf::from(CONFIG_FILE));
    }
  }
}

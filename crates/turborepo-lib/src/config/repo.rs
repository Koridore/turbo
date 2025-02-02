use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;
use config::Config;
use serde::{Deserialize, Serialize};

use super::{write_to_disk, MappedEnvironment};

const DEFAULT_API_URL: &str = "https://vercel.com/api";
const DEFAULT_LOGIN_URL: &str = "https://vercel.com";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoConfig {
    disk_config: RepoConfigValue,
    config: RepoConfigValue,
    path: PathBuf,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
struct RepoConfigValue {
    apiurl: Option<String>,
    loginurl: Option<String>,
    teamslug: Option<String>,
    teamid: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RepoConfigLoader {
    path: PathBuf,
    api: Option<String>,
    login: Option<String>,
    teamslug: Option<String>,
    environment: Option<HashMap<String, String>>,
}

impl RepoConfig {
    #[allow(dead_code)]
    pub fn api_url(&self) -> &str {
        self.config.apiurl.as_deref().unwrap_or(DEFAULT_API_URL)
    }

    #[allow(dead_code)]
    pub fn login_url(&self) -> &str {
        self.config.loginurl.as_deref().unwrap_or(DEFAULT_LOGIN_URL)
    }

    #[allow(dead_code)]
    pub fn team_slug(&self) -> Option<&str> {
        self.config.teamslug.as_deref()
    }

    #[allow(dead_code)]
    pub fn team_id(&self) -> Option<&str> {
        self.config.teamid.as_deref()
    }

    /// Sets the team id and clears the team slug, since it may have been from
    /// an old team
    #[allow(dead_code)]
    pub fn set_team_id(&mut self, team_id: Option<String>) -> Result<()> {
        self.disk_config.teamslug = None;
        self.config.teamslug = None;
        self.disk_config.teamid = team_id.clone();
        self.config.teamid = team_id;
        self.write_to_disk()
    }

    fn write_to_disk(&self) -> Result<()> {
        write_to_disk(&self.path, &self.disk_config)
    }
}

impl RepoConfigLoader {
    #[allow(dead_code)]
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            api: None,
            login: None,
            teamslug: None,
            environment: None,
        }
    }

    #[allow(dead_code)]
    pub fn with_api(mut self, api: Option<String>) -> Self {
        self.api = api;
        self
    }

    #[allow(dead_code)]
    pub fn with_login(mut self, login: Option<String>) -> Self {
        self.login = login;
        self
    }

    #[allow(dead_code)]
    pub fn with_team_slug(mut self, team_slug: Option<String>) -> Self {
        self.teamslug = team_slug;
        self
    }

    #[allow(dead_code)]
    pub fn with_environment(mut self, environment: Option<HashMap<String, String>>) -> Self {
        self.environment = environment;
        self
    }

    #[allow(dead_code)]
    pub fn load(self) -> Result<RepoConfig> {
        let Self {
            path,
            api,
            login,
            teamslug,
            environment,
        } = self;
        let raw_disk_config = Config::builder()
            .add_source(
                config::File::with_name(path.to_string_lossy().as_ref())
                    .format(config::FileFormat::Json)
                    .required(false),
            )
            .build()?;

        let has_teamslug_override = teamslug.is_some();

        let mut config: RepoConfigValue = Config::builder()
            .add_source(raw_disk_config.clone())
            .add_source(
                MappedEnvironment::with_prefix("turbo")
                    .source(environment)
                    .replace("api", "apiurl")
                    .replace("login", "loginurl")
                    .replace("team", "teamslug"),
            )
            .set_override_option("apiurl", api)?
            .set_override_option("loginurl", login)?
            .set_override_option("teamslug", teamslug)?
            // set teamid to none if teamslug present
            .build()?
            .try_deserialize()?;

        let disk_config: RepoConfigValue = raw_disk_config.try_deserialize()?;

        // If teamid was passed via command line flag we ignore team slug as it
        // might not match.
        if has_teamslug_override {
            config.teamid = None;
        }

        Ok(RepoConfig {
            disk_config,
            config,
            path,
        })
    }
}

#[cfg(test)]
mod test {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn test_repo_config_with_team_and_api_flags() -> Result<()> {
        let mut config_file = NamedTempFile::new()?;
        writeln!(&mut config_file, "{{\"teamId\": \"123\"}}")?;

        let config = RepoConfigLoader::new(config_file.path().to_path_buf())
            .with_team_slug(Some("my-team-slug".into()))
            .with_api(Some("http://my-login-url".into()))
            .load()?;

        assert_eq!(config.team_id(), None);
        assert_eq!(config.team_slug(), Some("my-team-slug"));
        assert_eq!(config.api_url(), "http://my-login-url");

        Ok(())
    }

    #[test]
    fn test_team_override_clears_id() -> Result<()> {
        let mut config_file = NamedTempFile::new()?;
        writeln!(&mut config_file, "{{\"teamId\": \"123\"}}")?;
        let loader = RepoConfigLoader::new(config_file.path().to_path_buf())
            .with_team_slug(Some("foo".into()));

        let config = loader.load()?;
        assert_eq!(config.team_slug(), Some("foo"));
        assert_eq!(config.team_id(), None);

        Ok(())
    }

    #[test]
    fn test_set_team_clears_id() -> Result<()> {
        let mut config_file = NamedTempFile::new()?;
        // We will never pragmatically write the "teamslug" field as camelCase,
        // but viper is case insensitive and we want to keep this functionality.
        writeln!(&mut config_file, "{{\"teamSlug\": \"my-team\"}}")?;
        let loader = RepoConfigLoader::new(config_file.path().to_path_buf());

        let mut config = loader.clone().load()?;
        config.set_team_id(Some("my-team-id".into()))?;

        let new_config = loader.load()?;
        assert_eq!(new_config.team_slug(), None);
        assert_eq!(new_config.team_id(), Some("my-team-id"));

        Ok(())
    }

    #[test]
    fn test_repo_env_variable() -> Result<()> {
        let mut config_file = NamedTempFile::new()?;
        writeln!(&mut config_file, "{{\"teamslug\": \"other-team\"}}")?;
        let login_url = "http://my-login-url";
        let api_url = "http://my-api";
        let team_id = "123";
        let team_slug = "my-team";
        let config = RepoConfigLoader::new(config_file.path().to_path_buf())
            .with_environment({
                let mut env = HashMap::new();
                env.insert("TURBO_API".into(), api_url.into());
                env.insert("TURBO_LOGIN".into(), login_url.into());
                env.insert("TURBO_TEAM".into(), team_slug.into());
                env.insert("TURBO_TEAMID".into(), team_id.into());
                Some(env)
            })
            .load()?;

        assert_eq!(config.login_url(), login_url);
        assert_eq!(config.api_url(), api_url);
        assert_eq!(config.team_id(), Some(team_id));
        assert_eq!(config.team_slug(), Some(team_slug));
        Ok(())
    }
}

use url::{self, Url};
use xshell::{Shell, cmd};

pub struct TestSupabaseCredentials {
    pub anon_key: String,
    pub email: String,
    pub password: String,
    pub supabase_api_url: url::Url,
}

/// Attempt to connect to `$WORSKAPCE-ROOT/test-supabase` supabase instance
///
/// # Errors
/// - cannot parse the `supabase status` data
pub fn get_supabase_credentials() -> eyre::Result<TestSupabaseCredentials> {
    let sh = Shell::new()?;
    sh.change_dir("./supabase");
    let status = cmd!(sh, "supabase status").read()?;

    // Parse the status output to extract required values
    let supabase_api_url = status
        .lines()
        .find(|line| line.contains("API URL:"))
        .and_then(|line| line.split("API URL:").nth(1))
        .map(|string| string.trim().to_owned())
        .ok_or_else(|| eyre::eyre!("Failed to parse API URL"))?;
    let supabase_api_url = Url::parse(&supabase_api_url)?;

    let anon_key = status
        .lines()
        .find(|line| line.contains("anon key:"))
        .and_then(|line| line.split("anon key:").nth(1))
        .map(|string| string.trim().to_owned())
        .ok_or_else(|| eyre::eyre!("Failed to parse anon key"))?;

    // Test credentials - in a real application, these would come from environment variables or config
    let email = "username@username.com".to_owned();
    let pass = "password".to_owned();

    Ok(TestSupabaseCredentials {
        supabase_api_url,
        anon_key,
        email,
        password: pass,
    })
}

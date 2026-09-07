use anyhow::{Context as _, Result, bail};
use clash_verge_logging::{Type, logging};
use std::process::{Command, Stdio};
use sysproxy::{Autoproxy, Sysproxy};
use system_configuration::{
    core_foundation::{
        base::{CFRelease, TCFType as _},
        dictionary::CFDictionary,
        string::{CFString, CFStringRef},
    },
    dynamic_store::SCDynamicStoreBuilder,
    preferences::SCPreferences,
    sys::network_configuration::{SCNetworkServiceCopy, SCNetworkServiceGetName},
};

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn applescript_quote(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn active_network_service() -> Result<String> {
    let store = SCDynamicStoreBuilder::new("clash-verge-sysproxy-auth")
        .build()
        .context("failed to access the macOS dynamic network store")?;
    let key = CFString::from_static_string("State:/Network/Global/IPv4");
    let state = store
        .get(key)
        .context("global IPv4 network state was not found")?
        .downcast_into::<CFDictionary>()
        .context("invalid global IPv4 network state")?;
    let primary_service_key = CFString::from_static_string("PrimaryService");
    let service_id = state
        .find(primary_service_key.as_CFTypeRef() as *const _)
        .map(|value| unsafe { CFString::wrap_under_get_rule(*value as _) })
        .context("primary network service was not found")?;

    let preferences = SCPreferences::default(&CFString::new("clash-verge-sysproxy-auth"));
    unsafe {
        let service = SCNetworkServiceCopy(preferences.as_concrete_TypeRef(), service_id.as_concrete_TypeRef());
        if service.is_null() {
            bail!("active network service was not found");
        }
        let name: CFStringRef = SCNetworkServiceGetName(service);
        let result = if name.is_null() {
            None
        } else {
            Some(CFString::wrap_under_get_rule(name).to_string())
        };
        CFRelease(service);
        result.context("active network service name was not found")
    }
}

pub async fn apply_via_service(sys: &Sysproxy, auto: &Autoproxy, auto_first: bool) -> Result<bool> {
    if !super::service::is_service_ipc_path_exists() {
        return Ok(false);
    }
    let service = tokio::task::spawn_blocking(active_network_service).await??;
    let config = clash_verge_service_ipc::SystemProxyConfig {
        service,
        host: sys.host.to_string(),
        port: sys.port,
        bypass: sys
            .bypass
            .split(',')
            .filter(|domain| !domain.is_empty())
            .map(str::to_owned)
            .collect(),
        system_enabled: sys.enable,
        auto_url: auto.url.to_string(),
        auto_enabled: auto.enable,
        auto_first,
    };
    let response = match clash_verge_service_ipc::apply_system_proxy(&config).await {
        Ok(response) => response,
        Err(error) => {
            logging!(warn, Type::Service, "System proxy service request unavailable: {error}");
            return Ok(false);
        }
    };
    if response.code > 0 {
        bail!(response.message);
    }
    Ok(true)
}

fn networksetup_command(args: &[String]) -> String {
    std::iter::once(shell_quote("/usr/sbin/networksetup"))
        .chain(args.iter().map(|arg| shell_quote(arg)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn push_proxy_commands(commands: &mut Vec<String>, service: &str, proxy: &Sysproxy) {
    let port = proxy.port.to_string();
    let state = if proxy.enable { "on" } else { "off" };
    for (set_command, state_command) in [
        ("-setsocksfirewallproxy", "-setsocksfirewallproxystate"),
        ("-setsecurewebproxy", "-setsecurewebproxystate"),
        ("-setwebproxy", "-setwebproxystate"),
    ] {
        commands.push(networksetup_command(&[
            set_command.into(),
            service.into(),
            proxy.host.clone(),
            port.clone(),
        ]));
        commands.push(networksetup_command(&[
            state_command.into(),
            service.into(),
            state.into(),
        ]));
    }

    let mut bypass = vec!["-setproxybypassdomains".into(), service.into()];
    bypass.extend(
        proxy
            .bypass
            .split(',')
            .filter(|domain| !domain.is_empty())
            .map(str::to_owned),
    );
    commands.push(networksetup_command(&bypass));
}

fn push_auto_proxy_commands(commands: &mut Vec<String>, service: &str, proxy: &Autoproxy) {
    commands.push(networksetup_command(&[
        "-setautoproxyurl".into(),
        service.into(),
        proxy.url.clone(),
    ]));
    commands.push(networksetup_command(&[
        "-setautoproxystate".into(),
        service.into(),
        if proxy.enable { "on" } else { "off" }.into(),
    ]));
}

/// Apply the complete proxy state through macOS' standard authorization dialog.
pub fn apply_with_authorization(sys: &Sysproxy, auto: &Autoproxy, auto_first: bool) -> Result<()> {
    let service = active_network_service()?;
    let mut sys_commands = Vec::new();
    let mut auto_commands = Vec::new();
    push_proxy_commands(&mut sys_commands, &service, sys);
    push_auto_proxy_commands(&mut auto_commands, &service, auto);

    let commands = if auto_first {
        auto_commands.into_iter().chain(sys_commands).collect::<Vec<_>>()
    } else {
        sys_commands.into_iter().chain(auto_commands).collect::<Vec<_>>()
    };
    let shell = commands.join(" && ");
    let script = format!(
        "do shell script \"{}\" with administrator privileges",
        applescript_quote(&shell)
    );

    let output = Command::new("/usr/bin/osascript")
        .args(["-e", &script])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("failed to open the administrator authorization dialog")?;

    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        bail!(if message.is_empty() {
            "administrator authorization was cancelled or denied".to_owned()
        } else {
            message
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{applescript_quote, shell_quote};

    #[test]
    fn quotes_shell_arguments() {
        assert_eq!(shell_quote("Bob's Wi-Fi"), "'Bob'\\''s Wi-Fi'");
    }

    #[test]
    fn quotes_applescript_strings() {
        assert_eq!(applescript_quote("a\\b\"c"), "a\\\\b\\\"c");
    }
}

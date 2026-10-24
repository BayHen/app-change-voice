use super::{DriverPackageInfo, DriverPackageState, VirtualAudioError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverInstallAction {
    Install,
    Repair,
    Upgrade,
    KeepInstalled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverReleasePackage {
    pub version: String,
    pub architecture: String,
    pub catalog_signed: bool,
    pub manifest_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverInstallationPlan {
    pub action: DriverInstallAction,
    pub requires_elevation: bool,
    pub requires_explicit_consent: bool,
}

/// Pure control-plane policy used by the app and installer integration.
///
/// It never launches an elevated process. Production installation is performed
/// only by the signed NSIS installer through Windows PnPUtil.
pub struct DriverInstallationService;

impl DriverInstallationService {
    pub fn plan(
        installed: Option<&DriverPackageInfo>,
        release: &DriverReleasePackage,
    ) -> Result<DriverInstallationPlan, VirtualAudioError> {
        if release.architecture != "x64" {
            return Err(VirtualAudioError::InvalidFormat(
                "virtual driver release must target x64".into(),
            ));
        }
        if !release.catalog_signed || !release.manifest_verified {
            return Err(VirtualAudioError::Transport(
                "virtual driver release package is not verified".into(),
            ));
        }

        let action = match installed {
            None => DriverInstallAction::Install,
            Some(info) if info.state == DriverPackageState::Problem => DriverInstallAction::Repair,
            Some(info) if version_is_older(&info.version, &release.version) => {
                DriverInstallAction::Upgrade
            }
            Some(_) => DriverInstallAction::KeepInstalled,
        };

        Ok(DriverInstallationPlan {
            action,
            requires_elevation: action != DriverInstallAction::KeepInstalled,
            requires_explicit_consent: action != DriverInstallAction::KeepInstalled,
        })
    }
}

fn version_is_older(installed: &str, release: &str) -> bool {
    fn parts(value: &str) -> Option<Vec<u32>> {
        value
            .split('.')
            .map(str::parse)
            .collect::<Result<_, _>>()
            .ok()
    }
    matches!((parts(installed), parts(release)), (Some(left), Some(right)) if left < right)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release() -> DriverReleasePackage {
        DriverReleasePackage {
            version: "0.1.0.0".into(),
            architecture: "x64".into(),
            catalog_signed: true,
            manifest_verified: true,
        }
    }

    #[test]
    fn rejects_unsigned_package() {
        let mut package = release();
        package.catalog_signed = false;
        assert!(DriverInstallationService::plan(None, &package).is_err());
    }

    #[test]
    fn plans_install_upgrade_repair_and_keep() {
        let info = |version: &str, state| DriverPackageInfo {
            published_inf_name: Some("oem42.inf".into()),
            provider: "VoiceMorph Studio".into(),
            version: version.into(),
            state,
            signed: true,
        };
        assert_eq!(
            DriverInstallationService::plan(None, &release())
                .unwrap()
                .action,
            DriverInstallAction::Install
        );
        assert_eq!(
            DriverInstallationService::plan(
                Some(&info("0.0.9.0", DriverPackageState::Installed)),
                &release()
            )
            .unwrap()
            .action,
            DriverInstallAction::Upgrade
        );
        assert_eq!(
            DriverInstallationService::plan(
                Some(&info("0.1.0.0", DriverPackageState::Problem)),
                &release()
            )
            .unwrap()
            .action,
            DriverInstallAction::Repair
        );
        assert_eq!(
            DriverInstallationService::plan(
                Some(&info("0.1.0.0", DriverPackageState::Installed)),
                &release()
            )
            .unwrap()
            .action,
            DriverInstallAction::KeepInstalled
        );
    }
}

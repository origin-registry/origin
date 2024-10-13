use cel::{CELIdentity, CELIdentityCertificate, CELRequest};
use cel_interpreter::{Context, Program, Value};
use log::{debug, error};
use x509_parser::prelude::X509Certificate;

mod cel;
mod client_action;

use crate::error::RegistryError;
use crate::registry::Registry;
pub use client_action::ClientAction;

#[derive(Clone, Debug, Default)]
pub struct ClientIdentity {
    pub cert_organizations: Vec<String>,
    pub cert_common_name: Vec<String>,
    pub credentials: Option<(String, String)>,
}

impl ClientIdentity {
    pub fn new() -> Self {
        ClientIdentity::default()
    }

    pub fn from_cert(cert: &X509Certificate) -> Result<ClientIdentity, RegistryError> {
        let subject = cert.subject();
        let cert_organizations = subject
            .iter_organization()
            .map(|o| o.as_str().map(String::from))
            .collect::<Result<Vec<String>, _>>()
            .map_err(|_| {
                RegistryError::Unauthorized(Some(
                    "Unable to parse provided certificate".to_string(),
                ))
            })?;
        let cert_common_name = subject
            .iter_common_name()
            .map(|o| o.as_str().map(String::from))
            .collect::<Result<Vec<String>, _>>()
            .map_err(|_| {
                RegistryError::Unauthorized(Some(
                    "Unable to parse provided certificate".to_string(),
                ))
            })?;

        Ok(ClientIdentity {
            cert_organizations,
            cert_common_name,
            credentials: None,
        })
    }

    pub fn set_credentials(&mut self, username: String, password: String) {
        self.credentials = Some((username, password));
    }

    pub fn can_do(&self, registry: &Registry, action: ClientAction) -> Result<(), RegistryError> {
        let identity_id = registry.validate_credentials(&self.credentials)?;

        let Some(namespace) = action.get_namespace() else {
            return Ok(());
        };

        let repository = registry
            .get_repository(&namespace)
            .ok_or_else(|| RegistryError::Unauthorized(Some("Repository not found".to_string())))?;

        let default_allow = registry.is_repository_policy_default_allow(&repository);
        debug!(
            "Default allow: {:?} for namespace: {:?} and repository: {:?}",
            default_allow, namespace, repository
        );

        let policies = registry.get_repository_policies(&repository);

        if let Some(policies) = policies {
            self.check_policies(action, identity_id, policies, default_allow)
        } else {
            error!(
                "Applying default policy to repository '{:?}' and action '{:?}'",
                repository, action
            );
            self.apply_default_policy(action, identity_id, default_allow)
        }
    }

    fn apply_default_policy(
        &self,
        action: ClientAction,
        identity_id: Option<String>,
        default_allow: bool,
    ) -> Result<(), RegistryError> {
        if default_allow {
            Ok(())
        } else {
            error!(
                "Default policy denied access: {:?} from {:?}",
                action, identity_id
            );
            Err(RegistryError::Unauthorized(Some(
                "Access denied (by policy)".to_string(),
            )))
        }
    }

    fn build_policy_context(&self, identity_id: &Option<String>, action: &ClientAction) -> Context {
        let request = CELRequest::from(action.clone());
        debug!("Policy context (request) : {:?}", request);

        let username = self
            .credentials
            .as_ref()
            .map(|(username, _)| username.clone());
        let certificate = CELIdentityCertificate::new(
            self.cert_organizations.clone(),
            self.cert_common_name.clone(),
        );
        let identity = CELIdentity::new(identity_id.clone(), username, certificate);
        debug!("Policy context (identity) : {:?}", identity);

        let mut context = Context::default();
        context.add_variable("request", &request).unwrap();
        context.add_variable("identity", &identity).unwrap();
        context
    }

    fn check_policies(
        &self,
        action: ClientAction,
        identity_id: Option<String>,
        policies: &[Program],
        default_allow: bool,
    ) -> Result<(), RegistryError> {
        let context = self.build_policy_context(&identity_id, &action);

        for policy in policies {
            let evaluation_result = policy.execute(&context).map_err(|e| {
                error!("Policy execution failed: {}", e);
                RegistryError::Unauthorized(Some("Policy execution failed".to_string()))
            })?;
            debug!("CEL program content {:?}", policy);
            debug!("CEL program evaluates to {:?}", evaluation_result);

            match evaluation_result {
                Value::Bool(true) if !default_allow => {
                    debug!("Policy matched, allowing access");
                    return Ok(());
                }
                Value::Bool(false) if default_allow => {
                    error!("Policy matched, denying access");
                    return Err(RegistryError::Unauthorized(Some(
                        "Access denied (by policy)".to_string(),
                    )));
                }
                Value::Bool(_) => {} // Not validated, continue checking
                _ => {
                    error!("Policy returned invalid value, denying access");
                    return Err(RegistryError::Unauthorized(Some(
                        "Access denied (by policy)".to_string(),
                    )));
                }
            }
        }

        debug!("No policy matched, applying default policy");
        self.apply_default_policy(action, identity_id, default_allow)
    }
}

use crate::{
    AppError, AppResult,
    config::{KubernetesConfig, KubernetesVolumeMount},
};
use anyhow::anyhow;
use k8s_openapi::api::batch::v1::Job;
use k8s_openapi::api::core::v1::{
    Container, EnvVar, HostPathVolumeSource, LocalObjectReference,
    PersistentVolumeClaimVolumeSource, PodSpec, PodTemplateSpec, Volume, VolumeMount,
};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::{
    Api, Client, ResourceExt,
    api::{DeleteParams, PostParams, Preconditions},
};
use std::collections::BTreeMap;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

/// Upper bound for waiting on a stale Job object to disappear after
/// requesting its deletion. Job objects leave etcd immediately (they carry
/// no finalizers by default), so hitting this timeout points at an
/// admission webhook or similar interference rather than normal operation.
const JOB_DELETION_TIMEOUT_SECS: u64 = 30;
const JOB_DELETION_POLL_INTERVAL_SECS: u64 = 1;
/// How often the single-flight loop re-reads after losing a race (UID
/// precondition conflict on delete, or 409 on create) before giving up.
const LAUNCH_CONTENTION_RETRIES: usize = 3;
const MANAGED_BY_LABEL: &str = "app.kubernetes.io/managed-by";

#[derive(Clone)]
pub struct KubernetesClient {
    client: Client,
    config: KubernetesConfig,
}

impl std::fmt::Debug for KubernetesClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KubernetesClient")
            .field("namespace", &self.config.namespace)
            .field("job_name", &self.config.job_name)
            .finish_non_exhaustive()
    }
}

impl KubernetesClient {
    /// Resolves cluster credentials the same way kubectl does: the in-cluster
    /// service account first (the intended deployment mode), then
    /// `$KUBECONFIG` / `~/.kube/config` for processes running outside the
    /// cluster.
    pub async fn new(config: KubernetesConfig) -> AppResult<Self> {
        let client = Client::try_default()
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?;
        Ok(Self { client, config })
    }

    /// Creates the asset-extraction Job for the given versions. The call is
    /// fire-and-forget like the Docker implementation it replaces: pod
    /// scheduling and retries belong to the cluster, completion is observed
    /// through the shared output volume, not through this API.
    pub async fn launch_container(
        &self,
        client_version: &str,
        res_version: &str,
        prev_client_version: &str,
        prev_res_version: &str,
        include: Option<&str>,
        exclude: Option<&str>,
    ) -> AppResult<String> {
        let job_name = self.config.job_name.as_str();
        let jobs: Api<Job> = Api::namespaced(self.client.clone(), &self.config.namespace);

        // Job objects are immutable, so a finished Job must be deleted before
        // it can be recreated under the same name. The fixed name doubles as
        // the single-flight lock: a Job that has not reached a terminal
        // condition blocks the launch, matching the old inspect-container
        // guard.
        //
        // Both the delete and the create can lose a race against a concurrent
        // launch of the same fixed name. The delete carries a UID precondition
        // so a losing request can never delete the winner's fresh Job (it
        // would otherwise remove whatever currently carries the name); on any
        // conflict the loop re-reads and re-decides instead of returning a
        // stale success.
        for attempt in 0..=LAUNCH_CONTENTION_RETRIES {
            match jobs.get(job_name).await {
                Ok(job) => {
                    if job_is_active(&job) {
                        return Err(AppError::ExternalService(anyhow!(
                            "Job {job_name} is already running"
                        )));
                    }

                    warn!("Job {job_name} exists but is not running, removing it");
                    let delete_params = DeleteParams {
                        preconditions: Some(Preconditions {
                            uid: job.uid(),
                            resource_version: None,
                        }),
                        ..Default::default()
                    };
                    match jobs.delete(job_name, &delete_params).await {
                        Ok(_) => {}
                        Err(kube::Error::Api(err)) if err.code == 409 || err.code == 404 => {
                            warn!(
                                "Job {job_name} changed under us on attempt {}, re-reading",
                                attempt + 1
                            );
                            continue;
                        }
                        Err(err) => return Err(AppError::ExternalService(err.into())),
                    }
                    wait_for_job_deleted(&jobs, job_name).await?;
                    info!("Job deleted: {job_name}");
                }
                Err(kube::Error::Api(err)) if err.code == 404 => {
                    info!("Job {job_name} does not exist, will create new one");
                }
                Err(err) => return Err(AppError::ExternalService(err.into())),
            }

            // Image pulls (with backoff and private-registry auth via
            // imagePullSecrets) are handled by kubelet, so there is no
            // client-side pull loop here.
            let job = build_job(
                &self.config,
                client_version,
                res_version,
                prev_client_version,
                prev_res_version,
                include,
                exclude,
            )?;
            match jobs.create(&PostParams::default(), &job).await {
                Ok(_) => {
                    info!("Job started successfully: {job_name}");
                    return Ok(job_name.to_string());
                }
                // A concurrent launch created the Job between our check and
                // create; re-read to report what actually occupies the name.
                Err(kube::Error::Api(err)) if err.code == 409 => {
                    warn!(
                        "Job {job_name} was created concurrently on attempt {}, re-reading",
                        attempt + 1
                    );
                }
                Err(err) => return Err(AppError::ExternalService(err.into())),
            }
        }

        Err(AppError::ExternalService(anyhow!(
            "Job {job_name} kept changing under us; concurrent launch in progress?"
        )))
    }
}

/// Whether the Job still occupies the single-flight slot. Only a terminal
/// condition (`Complete` or `Failed` = True) releases it: `active` already
/// drops to 0 while the last pod is still terminating, and a Job the
/// controller has not observed yet carries no conditions at all. Everything
/// non-terminal keeps blocking, so a launch can never delete a Job whose
/// output is still settling.
fn job_is_active(job: &Job) -> bool {
    let terminal = job
        .status
        .as_ref()
        .and_then(|status| status.conditions.as_ref())
        .is_some_and(|conditions| {
            conditions.iter().any(|condition| {
                (condition.type_ == "Complete" || condition.type_ == "Failed")
                    && condition.status == "True"
            })
        });
    !terminal
}

async fn wait_for_job_deleted(jobs: &Api<Job>, job_name: &str) -> AppResult<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(JOB_DELETION_TIMEOUT_SECS);
    while tokio::time::Instant::now() < deadline {
        match jobs.get(job_name).await {
            Err(kube::Error::Api(err)) if err.code == 404 => return Ok(()),
            Ok(_) => {}
            Err(err) => return Err(AppError::ExternalService(err.into())),
        }
        sleep(Duration::from_secs(JOB_DELETION_POLL_INTERVAL_SECS)).await;
    }
    Err(AppError::ExternalService(anyhow!(
        "Job {job_name} was not deleted within {JOB_DELETION_TIMEOUT_SECS}s"
    )))
}

fn build_job(
    config: &KubernetesConfig,
    client_version: &str,
    res_version: &str,
    prev_client_version: &str,
    prev_res_version: &str,
    include: Option<&str>,
    exclude: Option<&str>,
) -> AppResult<Job> {
    // The image's ENTRYPOINT is preserved; only its CMD is overridden, which
    // is exactly what the Docker implementation's `cmd` did.
    let mut args = vec![
        client_version.to_string(),
        res_version.to_string(),
        "-c".to_string(),
        prev_client_version.to_string(),
        "-r".to_string(),
        prev_res_version.to_string(),
    ];
    if let Some(include) = include {
        args.push("-i".to_string());
        args.push(include.to_string());
    }
    if let Some(exclude) = exclude {
        args.push("-e".to_string());
        args.push(exclude.to_string());
    }

    let env = parse_env_vars(config.env_vars.as_deref().unwrap_or(&[]))?;
    let (volumes, volume_mounts) = build_volumes(config.volume_mounts.as_deref().unwrap_or(&[]));

    let image_pull_secrets = config.image_pull_secret.as_ref().map(|secret_name| {
        vec![LocalObjectReference {
            name: secret_name.clone(),
        }]
    });

    Ok(Job {
        metadata: ObjectMeta {
            name: Some(config.job_name.clone()),
            namespace: Some(config.namespace.clone()),
            labels: Some(BTreeMap::from([(
                MANAGED_BY_LABEL.to_string(),
                "ak-asset-storage".to_string(),
            )])),
            ..Default::default()
        },
        spec: Some(k8s_openapi::api::batch::v1::JobSpec {
            // Launch exactly once, like `docker run` did; the launch endpoint
            // stays the retry knob for operators.
            backoff_limit: Some(0),
            template: PodTemplateSpec {
                // The label must land on the Job's *pods*, not only the Job
                // object, so cluster-side policy (e.g. a NetworkPolicy that
                // scopes extractor egress) can select them.
                metadata: Some(ObjectMeta {
                    labels: Some(BTreeMap::from([(
                        MANAGED_BY_LABEL.to_string(),
                        "ak-asset-storage".to_string(),
                    )])),
                    ..Default::default()
                }),
                spec: Some(PodSpec {
                    restart_policy: Some("Never".to_string()),
                    image_pull_secrets,
                    containers: vec![Container {
                        name: config.job_name.clone(),
                        image: Some(config.image_url.clone()),
                        args: Some(args),
                        env: (!env.is_empty()).then_some(env),
                        volume_mounts: (!volume_mounts.is_empty()).then_some(volume_mounts),
                        ..Default::default()
                    }],
                    volumes: (!volumes.is_empty()).then_some(volumes),
                    ..Default::default()
                }),
            },
            ..Default::default()
        }),
        ..Default::default()
    })
}

fn parse_env_vars(entries: &[String]) -> AppResult<Vec<EnvVar>> {
    entries
        .iter()
        .map(|entry| {
            let (name, value) = entry.split_once('=').ok_or_else(|| {
                AppError::Application(anyhow!(
                    "invalid env var {entry:?} in torappu.kubernetes.env_vars: expected KEY=VALUE"
                ))
            })?;
            Ok(EnvVar {
                name: name.to_string(),
                value: Some(value.to_string()),
                ..Default::default()
            })
        })
        .collect()
}

/// Builds parallel `volumes` / `volumeMounts` lists from the config. Volume
/// names are positional (`vol-0`, `vol-1`, …) since the config only pairs one
/// source with one mount path.
fn build_volumes(mounts: &[KubernetesVolumeMount]) -> (Vec<Volume>, Vec<VolumeMount>) {
    mounts
        .iter()
        .enumerate()
        .map(|(index, mount)| {
            let volume_name = format!("vol-{index}");
            let mut volume = Volume {
                name: volume_name.clone(),
                ..Default::default()
            };
            if let Some(claim_name) = &mount.pvc {
                volume.persistent_volume_claim = Some(PersistentVolumeClaimVolumeSource {
                    claim_name: claim_name.clone(),
                    read_only: None,
                });
            } else {
                volume.host_path = Some(HostPathVolumeSource {
                    path: mount.host_path.clone().unwrap_or_default(),
                    type_: None,
                });
            }
            (
                volume,
                VolumeMount {
                    name: volume_name,
                    mount_path: mount.mount_path.clone(),
                    ..Default::default()
                },
            )
        })
        .unzip()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KubernetesVolumeMount;
    use k8s_openapi::api::batch::v1::JobStatus;

    fn config() -> KubernetesConfig {
        KubernetesConfig {
            image_url: "example.com/extractor:latest".to_string(),
            namespace: "ak-asset-storage".to_string(),
            job_name: "ak-asset-job".to_string(),
            image_pull_secret: Some("registry-cred".to_string()),
            env_vars: Some(vec!["TZ=Asia/Shanghai".to_string()]),
            volume_mounts: Some(vec![KubernetesVolumeMount {
                mount_path: "/app/data".to_string(),
                pvc: Some("asset-data".to_string()),
                host_path: None,
            }]),
        }
    }

    #[test]
    fn job_builds_command_args_in_launch_contract_order() {
        let job = build_job(
            &config(),
            "2.7.41",
            "26-05-27",
            "2.7.31",
            "26-05-20",
            Some("arts/*"),
            Some("scenes/x"),
        )
        .expect("job builds");

        let container = &job
            .spec
            .as_ref()
            .unwrap()
            .template
            .spec
            .as_ref()
            .unwrap()
            .containers[0];
        assert_eq!(
            container.args.as_deref().unwrap(),
            [
                "2.7.41", "26-05-27", "-c", "2.7.31", "-r", "26-05-20", "-i", "arts/*", "-e",
                "scenes/x"
            ]
        );
        assert_eq!(
            container.image.as_deref(),
            Some("example.com/extractor:latest")
        );
        let env = container.env.as_ref().unwrap();
        assert_eq!(env[0].name, "TZ");
        assert_eq!(env[0].value.as_deref(), Some("Asia/Shanghai"));
    }

    #[test]
    fn job_wires_pull_secret_and_volumes() {
        let job = build_job(&config(), "c", "r", "pc", "pr", None, None).expect("job builds");
        let spec = job.spec.as_ref().unwrap();
        let pod_spec = spec.template.spec.as_ref().unwrap();

        assert_eq!(spec.backoff_limit, Some(0));
        assert_eq!(pod_spec.restart_policy.as_deref(), Some("Never"));
        assert_eq!(
            pod_spec.image_pull_secrets.as_ref().unwrap()[0].name,
            "registry-cred"
        );
        assert_eq!(
            pod_spec.volumes.as_ref().unwrap()[0]
                .persistent_volume_claim
                .as_ref()
                .unwrap()
                .claim_name,
            "asset-data"
        );
        assert_eq!(
            pod_spec.containers[0].volume_mounts.as_ref().unwrap()[0].mount_path,
            "/app/data"
        );
    }

    #[test]
    fn job_metadata_targets_the_configured_namespace() {
        let job = build_job(&config(), "c", "r", "pc", "pr", None, None).expect("job builds");
        let metadata = &job.metadata;
        assert_eq!(metadata.name.as_deref(), Some("ak-asset-job"));
        assert_eq!(metadata.namespace.as_deref(), Some("ak-asset-storage"));
        assert_eq!(
            metadata.labels.as_ref().unwrap()[MANAGED_BY_LABEL],
            "ak-asset-storage"
        );
        // The label must also reach the Job's pods so cluster-side policy
        // (e.g. NetworkPolicy) can select them.
        assert_eq!(
            job.spec
                .as_ref()
                .unwrap()
                .template
                .metadata
                .as_ref()
                .unwrap()
                .labels
                .as_ref()
                .unwrap()[MANAGED_BY_LABEL],
            "ak-asset-storage"
        );
    }

    #[test]
    fn malformed_env_var_rejects_the_launch() {
        let mut config = config();
        config.env_vars = Some(vec!["MISSING_SEPARATOR".to_string()]);
        let error = build_job(&config, "c", "r", "pc", "pr", None, None).unwrap_err();
        assert!(error.to_string().contains("MISSING_SEPARATOR"), "{error:?}");
    }

    #[test]
    fn unreconciled_and_active_jobs_count_as_running() {
        let unreconciled: Job = serde_json::from_value(serde_json::json!({
            "metadata": {"name": "j"},
        }))
        .unwrap();
        assert!(job_is_active(&unreconciled));

        let created_not_started: Job = serde_json::from_value(serde_json::json!({
            "metadata": {"name": "j"},
            "status": {"active": null}
        }))
        .unwrap();
        assert!(job_is_active(&created_not_started));

        let running: Job = serde_json::from_value(serde_json::json!({
            "metadata": {"name": "j"},
            "status": {"active": 1, "startTime": "2026-01-01T00:00:00Z"}
        }))
        .unwrap();
        assert!(job_is_active(&running));
    }

    #[test]
    fn only_terminal_conditions_release_the_single_flight_slot() {
        // active already drops to 0 while the last pod is still terminating
        // and the controller has not stamped a terminal condition yet: the
        // Job must keep blocking (reproduced as terminating=1 being deleted).
        let still_settling: Job = serde_json::from_value(serde_json::json!({
            "metadata": {"name": "j"},
            "status": {"active": 0, "startTime": "2026-01-01T00:00:00Z", "succeeded": 1}
        }))
        .unwrap();
        assert!(job_is_active(&still_settling));

        let complete: Job = serde_json::from_value(serde_json::json!({
            "metadata": {"name": "j"},
            "status": {
                "active": 0,
                "startTime": "2026-01-01T00:00:00Z",
                "succeeded": 1,
                "conditions": [{"type": "Complete", "status": "True"}]
            }
        }))
        .unwrap();
        assert!(!job_is_active(&complete));

        let failed: Job = serde_json::from_value(serde_json::json!({
            "metadata": {"name": "j"},
            "status": {
                "active": 0,
                "startTime": "2026-01-01T00:00:00Z",
                "failed": 1,
                "conditions": [{"type": "Failed", "status": "True"}]
            }
        }))
        .unwrap();
        assert!(!job_is_active(&failed));

        // A condition that is present but not True does not count.
        let failure_in_flight: Job = serde_json::from_value(serde_json::json!({
            "metadata": {"name": "j"},
            "status": {
                "active": 0,
                "startTime": "2026-01-01T00:00:00Z",
                "conditions": [{"type": "Failed", "status": "False"}]
            }
        }))
        .unwrap();
        assert!(job_is_active(&failure_in_flight));
    }

    #[test]
    fn status_without_start_time_deserializes_like_the_api_sends_it() {
        // Guards the job_is_active predicate against JobStatus shape changes.
        let status: JobStatus = serde_json::from_value(serde_json::json!({
            "active": 1,
            "startTime": "2026-01-01T00:00:00Z"
        }))
        .unwrap();
        assert_eq!(status.active, Some(1));
        assert!(status.start_time.is_some());
    }
}

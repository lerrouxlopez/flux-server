use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use once_cell::sync::OnceCell;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

static PROM_HANDLE: OnceCell<PrometheusHandle> = OnceCell::new();

pub fn init() {
    // Logs
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let log_format = std::env::var("LOG_FORMAT").unwrap_or_else(|_| "pretty".to_string());

    let fmt_layer = if log_format.trim().eq_ignore_ascii_case("json") {
        tracing_subscriber::fmt::layer().json()
    } else {
        tracing_subscriber::fmt::layer()
    };

    tracing_subscriber::registry().with(filter).with(fmt_layer).init();

    // Metrics (Prometheus)
    // Exposed via API apps at /metrics (they call prometheus_handle().render()).
    // Keep installation idempotent (tests may init multiple times).
    let _ = PROM_HANDLE.get_or_try_init(|| {
        PrometheusBuilder::new()
            .set_buckets_for_metric(
                "http_server_request_duration_seconds",
                &{
                    // 5ms .. 10s
                    let mut b = Vec::new();
                    let mut v = 0.005_f64;
                    while v < 10.0 {
                        b.push(v);
                        v *= 1.7;
                    }
                    b
                },
            )?
            .install_recorder()
    });
}

pub fn prometheus_handle() -> Option<&'static PrometheusHandle> {
    PROM_HANDLE.get()
}

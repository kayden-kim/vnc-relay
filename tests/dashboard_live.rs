//! Real-process dashboard acceptance through a small CDP client and a transparent
//! test-side forwarding proxy. Injected faults are not product server failures.
//! Each function below covers its whole named row; manual DASH-015/023 are separate.

use std::sync::{Mutex, PoisonError};

#[path = "dashboard_live/cross.rs"]
mod cross;
#[path = "dashboard_live/detail.rs"]
mod detail;
#[path = "common/dashboard_live.rs"]
mod live;
#[path = "dashboard_live/login.rs"]
mod login;
#[path = "dashboard_live/overview.rs"]
mod overview;

// Several scenarios hold multiple browsers and real listeners. Serialize this
// binary's rows to preserve their timing evidence on both supported OS gates.
static SCENARIO: Mutex<()> = Mutex::new(());

fn run(scenario: fn()) {
    let _guard = SCENARIO.lock().unwrap_or_else(PoisonError::into_inner);
    scenario();
}

/// spec: dashboard.md:DASH-001
#[test]
fn dashboard_001_real_overview_data_and_unavailable_values() {
    run(overview::overview_known_data);
}

/// spec: dashboard.md:DASH-002
#[test]
fn dashboard_002_real_empty_lists() {
    run(overview::overview_empty_lists);
}

/// spec: dashboard.md:DASH-003
#[test]
fn dashboard_003_first_and_stale_failures_on_both_screens() {
    run(cross::failed_reads_distinguish_empty_and_stale);
}

/// spec: dashboard.md:DASH-004
#[test]
fn dashboard_004_explicit_refresh_and_success_times_on_both_screens() {
    run(cross::explicit_refresh_recovers_only_on_success);
}

/// spec: dashboard.md:DASH-005
#[test]
fn dashboard_005_real_delays_do_not_overlap_or_overwrite_routes() {
    run(cross::delayed_queries_do_not_overlap_or_overwrite_routes);
}

/// spec: dashboard.md:DASH-006
#[test]
fn dashboard_006_selected_real_session_facts_and_return() {
    run(detail::selected_detail_matches_real_session_and_returns_to_list);
}

/// spec: dashboard.md:DASH-007
#[test]
fn dashboard_007_real_disappearance_before_and_during_detail() {
    run(detail::disappeared_detail_before_read_or_during_refresh_is_gone);
}

/// spec: dashboard.md:DASH-008
#[test]
fn dashboard_008_real_300_second_idle_expiry_on_both_screens() {
    run(cross::automatic_polls_expire_and_unauthorized_reads_clear_screens);
}

/// spec: dashboard.md:DASH-009
#[test]
fn dashboard_009_cancel_and_close_restore_focus_without_termination() {
    run(detail::cancel_and_escape_preserve_real_session_and_restore_focus);
}

/// spec: dashboard.md:DASH-010
#[test]
fn dashboard_010_real_termination_blocks_duplicates_and_refreshes_list() {
    run(detail::confirmed_termination_blocks_duplicates_then_refreshes_list);
}

/// spec: dashboard.md:DASH-011
#[test]
fn dashboard_011_natural_close_around_confirmation_preserves_other_session() {
    run(detail::natural_close_before_or_after_confirmation_never_targets_another_session);
}

/// spec: dashboard.md:DASH-012
#[test]
fn dashboard_012_injected_failure_is_not_termination_completion() {
    run(detail::injected_termination_failure_is_not_completion_and_offers_status_read);
}

/// spec: dashboard.md:DASH-013
#[test]
fn dashboard_013_lost_real_termination_response_reads_before_resending() {
    run(detail::lost_real_termination_response_is_unknown_and_reads_before_any_resend);
}

/// spec: dashboard.md:DASH-014
#[test]
fn dashboard_014_unauthenticated_direct_detail_exposes_no_data() {
    run(detail::unauthenticated_direct_detail_exposes_no_data_or_termination);
}

/// spec: dashboard.md:DASH-016
#[test]
fn dashboard_016_real_state_changes_appear_on_next_poll() {
    run(cross::relay_changes_appear_on_next_poll);
}

/// spec: dashboard.md:DASH-017
#[test]
fn dashboard_017_real_login_opens_overview_and_queries() {
    run(login::correct_credentials_open_overview_and_queries);
}

/// spec: dashboard.md:DASH-018
#[test]
fn dashboard_018_empty_wrong_and_rate_limited_real_login() {
    run(login::empty_wrong_and_locked_login_hide_protected_data);
}

/// spec: dashboard.md:DASH-019
#[test]
fn dashboard_019_logout_success_and_lost_response_on_both_screens() {
    run(cross::logout_clears_both_screens_and_preserves_relay);
}

/// spec: dashboard.md:DASH-020
#[test]
fn dashboard_020_real_eof_error_admin_and_empty_history() {
    run(overview::overview_recent_closed);
}

/// spec: dashboard.md:DASH-021
#[test]
fn dashboard_021_real_pending_counts_and_resource_usage() {
    run(overview::overview_connection_counts);
}

/// spec: dashboard.md:DASH-022
#[test]
fn dashboard_022_real_configured_and_default_polling_on_both_screens() {
    run(cross::configured_and_default_poll_intervals);
}

/// spec: dashboard.md:DASH-024
#[test]
fn dashboard_024_browser_preferences_on_all_three_screens() {
    run(cross::browser_preferences_select_language_on_all_screens);
}

/// spec: dashboard.md:DASH-025
#[test]
fn dashboard_025_saved_language_survives_real_navigation_and_preserves_values() {
    run(cross::saved_language_survives_all_screens);
}

/// spec: dashboard.md:DASH-026
#[test]
fn dashboard_026_malformed_ids_send_no_query_and_differ_from_not_found() {
    run(detail::malformed_detail_ids_make_no_api_request_and_differ_from_real_not_found);
}

/// spec: dashboard.md:DASH-027
#[test]
fn dashboard_027_injected_login_failures_block_duplicates_and_allow_retry() {
    run(login::login_transport_failures_are_distinct_and_block_duplicates);
}

#[test]
fn dashboard_proxy_preserves_an_idle_browser_preconnection() {
    run(|| {
        use std::io::Write;
        use std::time::Duration;

        let server = live::Server::start("proxy-preconnect", None);
        let accepted = server.proxy.accepted_count();
        let mut stream = live::connect(server.proxy.port);
        live::wait_for(
            "accepted proxy worker reaches request read",
            live::LIMIT,
            || server.proxy.accepted_count() > accepted,
        );
        // The worker has acknowledged entry into its request-read path.
        // Windows used to inherit nonblocking mode and close it with 10035.
        std::thread::sleep(Duration::from_millis(100));
        live::must(
            stream
                .write_all(b"GET /login/ HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"),
        );
        let reply = live::read_reply(stream);
        assert_eq!(reply.status, 200);
        assert!(reply.body.contains("login-root"));
        live::wait_for("preconnected HTTP exchange completes", live::LIMIT, || {
            server.proxy.traffic().iter().any(|item| {
                item.path == "/login/"
                    && item.forwarded
                    && !item.injected
                    && item.status == Some(200)
                    && item.completed.is_some()
                    && item.error.is_none()
            })
        });
    });
}

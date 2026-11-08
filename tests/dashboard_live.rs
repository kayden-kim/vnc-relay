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
#[path = "dashboard_live/logs.rs"]
mod logs;
#[path = "dashboard_live/logs_accessibility.rs"]
mod logs_accessibility;
#[path = "dashboard_live/logs_download.rs"]
mod logs_download;
#[path = "dashboard_live/logs_states.rs"]
mod logs_states;
#[path = "dashboard_live/overview.rs"]
mod overview;
#[path = "dashboard_live/settings.rs"]
mod settings;
#[path = "dashboard_live/settings_edit.rs"]
mod settings_edit;
#[path = "dashboard_live/settings_recovery.rs"]
mod settings_recovery;
#[path = "dashboard_live/settings_view.rs"]
mod settings_view;
#[path = "dashboard_live/transports.rs"]
mod transports;

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

/// spec: dashboard.md:DASH-028
#[test]
fn dashboard_028_settings_navigation_and_protected_direct_routes() {
    run(settings_view::settings_navigation_and_protected_direct_routes);
}

/// spec: dashboard.md:DASH-029
#[test]
fn dashboard_029_settings_view_displays_all_effective_values_in_contract_order() {
    run(settings_view::settings_view_displays_all_effective_values_in_contract_order);
}

/// spec: dashboard.md:DASH-030
#[test]
fn dashboard_030_in_effect_and_pending_settings_allow_edit_without_polling() {
    run(settings_view::in_effect_and_pending_settings_allow_edit_without_polling);
}

/// spec: dashboard.md:DASH-031
#[test]
fn dashboard_031_restart_pending_settings_show_keys_and_block_edit() {
    run(settings_view::restart_pending_settings_show_keys_and_block_edit);
}

/// spec: dashboard.md:DASH-032
#[test]
fn dashboard_032_invalid_and_unreadable_settings_keep_effective_values_and_localize_diagnostics() {
    run(settings_view::invalid_and_unreadable_settings_keep_effective_values_and_localize_diagnostics);
}

/// spec: dashboard.md:DASH-033
#[test]
fn dashboard_033_settings_without_reload_history_show_no_history() {
    run(settings_view::settings_without_reload_history_show_no_history);
}

/// spec: dashboard.md:DASH-034
#[test]
fn dashboard_034_disabled_edit_direct_route_keeps_view_without_inputs() {
    run(settings_view::disabled_edit_direct_route_keeps_view_without_inputs);
}

/// spec: dashboard.md:DASH-035
#[test]
fn dashboard_035_settings_edit_exposes_only_enabled_editable_keys() {
    run(settings_edit::settings_edit_exposes_only_enabled_editable_keys);
}

/// spec: dashboard.md:DASH-036
#[test]
fn dashboard_036_invalid_settings_inputs_focus_first_error_without_patch() {
    run(settings_edit::invalid_settings_inputs_focus_first_error_without_patch);
}

/// spec: dashboard.md:DASH-037
#[test]
fn dashboard_037_settings_confirmation_shows_diffs_empty_list_warning_and_timing() {
    run(settings_edit::settings_confirmation_shows_diffs_empty_list_warning_and_timing);
}

/// spec: dashboard.md:DASH-038
#[test]
fn dashboard_038_settings_confirmation_cancel_and_escape_preserve_input_and_focus() {
    run(settings_edit::settings_confirmation_cancel_and_escape_preserve_input_and_focus);
}

/// spec: dashboard.md:DASH-039
#[test]
fn dashboard_039_confirmed_settings_patch_sends_revision_csrf_and_shows_applied_count() {
    run(settings_edit::confirmed_settings_patch_sends_revision_csrf_and_shows_applied_count);
}

/// spec: dashboard.md:DASH-040
#[test]
fn dashboard_040_server_settings_diagnostics_preserve_input_and_map_original_lines() {
    run(settings_edit::server_settings_diagnostics_preserve_input_and_map_original_lines);
}

/// spec: dashboard.md:DASH-041
#[test]
fn dashboard_041_settings_conflict_keeps_input_and_never_resends_automatically() {
    run(settings_recovery::settings_conflict_keeps_input_and_never_resends_automatically);
}

/// spec: management.md:MGT-043
/// spec: dashboard.md:DASH-042
#[test]
fn dashboard_042_config_conflict_reload_discards_input_and_uses_fresh_revision() {
    run(settings_recovery::config_conflict_reload_discards_input_and_uses_fresh_revision);
}

/// spec: dashboard.md:DASH-043
#[test]
fn dashboard_043_disabled_restart_pending_and_invalid_patch_results_remove_edit_actions() {
    run(settings_recovery::disabled_restart_pending_and_invalid_patch_results_remove_edit_actions);
}

/// spec: dashboard.md:DASH-044
#[test]
fn dashboard_044_config_write_failure_keeps_input_without_success_notice() {
    run(settings_recovery::config_write_failure_keeps_input_without_success_notice);
}

/// spec: dashboard.md:DASH-045
#[test]
fn dashboard_045_explicit_settings_retry_after_write_repair_saves_once() {
    run(settings_recovery::explicit_settings_retry_after_write_repair_saves_once);
}

/// spec: management.md:MGT-040
/// spec: dashboard.md:DASH-046
#[test]
fn dashboard_046_config_response_loss_and_timeout_require_get_before_retry() {
    run(settings_recovery::config_response_loss_and_timeout_require_get_before_retry);
}

/// spec: dashboard.md:DASH-047
#[test]
fn dashboard_047_unknown_edit_result_is_checked_with_get_without_patch_replay() {
    run(settings_recovery::unknown_edit_result_is_checked_with_get_without_patch_replay);
}

/// spec: management.md:MGT-042
/// spec: dashboard.md:DASH-048
#[test]
fn dashboard_048_config_query_failures_keep_stale_values_and_never_show_empty_success() {
    run(settings_view::config_query_failures_keep_stale_values_and_never_show_empty_success);
}

/// spec: dashboard.md:DASH-049
#[test]
fn dashboard_049_explicit_settings_refresh_after_repair_updates_values_revision_and_time() {
    run(settings_view::explicit_settings_refresh_after_repair_updates_values_revision_and_time);
}

/// spec: dashboard.md:DASH-050
#[test]
fn dashboard_050_settings_screens_preserve_input_and_accessibility_in_both_languages_and_widths() {
    run(settings_recovery::settings_screens_preserve_input_and_accessibility_in_both_languages_and_widths);
}

/// spec: dashboard.md:DASH-051
#[test]
fn dashboard_051_settings_expiry_and_logout_remove_protected_values_and_leave_relays_alive() {
    run(settings_recovery::settings_expiry_and_logout_remove_protected_values_and_leave_relays_alive);
}

/// spec: dashboard.md:DASH-052
#[test]
fn dashboard_052_unchanged_settings_return_without_patch_and_empty_applied_shows_zero() {
    run(settings_edit::unchanged_settings_return_without_patch_and_empty_applied_shows_zero);
}

/// spec: dashboard.md:DASH-053
#[test]
fn dashboard_053_leaving_settings_edit_discards_input_without_confirmation_or_patch() {
    run(settings_recovery::leaving_settings_edit_discards_input_without_confirmation_or_patch);
}

/// spec: dashboard.md:DASH-054
#[test]
fn dashboard_054_real_logs_first_load_and_entry_fields() {
    run(logs::first_load_and_all_entry_fields);
}

/// spec: dashboard.md:DASH-055
#[test]
fn dashboard_055_real_level_event_filters_and_obsolete_reply() {
    run(logs::level_event_filters_and_obsolete_reply);
}

/// spec: dashboard.md:DASH-056
#[test]
fn dashboard_056_real_unicode_value_search_and_boundaries() {
    run(logs::unicode_value_search_and_boundaries);
}

/// spec: dashboard.md:DASH-057
#[test]
fn dashboard_057_real_polling_pages_pause_no_overlap_and_eviction() {
    run(logs::polling_pages_pause_no_overlap_and_eviction);
}

/// spec: dashboard.md:DASH-058
#[test]
fn dashboard_058_real_older_pages_match_cursor_results() {
    run(logs::older_pages_match_real_cursor_results);
}

/// spec: dashboard.md:DASH-059
#[test]
fn dashboard_059_real_session_link_invalid_id_and_retention_limit() {
    run(logs::session_link_invalid_id_clear_and_retention_limit);
}

/// spec: dashboard.md:DASH-060
#[test]
fn dashboard_060_real_json_lines_download_and_no_partial_files() {
    run(logs_download::complete_json_lines_snapshot_and_no_partial_files);
}

/// spec: dashboard.md:DASH-061
#[test]
fn dashboard_061_real_disabled_failures_recovery_and_300_second_expiry() {
    run(logs_states::disabled_empty_failures_recovery_and_real_idle_expiry);
}

/// spec: dashboard.md:DASH-062
#[test]
fn dashboard_062_real_transport_matrix_and_forwarded_addresses() {
    run(transports::real_transport_matrix_and_forwarded_addresses);
}

/// spec: dashboard.md:DASH-063
#[test]
fn dashboard_063_real_logs_keyboard_languages_widths_and_preservation() {
    run(logs_accessibility::keyboard_languages_widths_and_state_preservation);
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

#![allow(clippy::too_many_arguments)]
#![allow(linker_messages)]

mod commands;
pub mod db;
pub mod domain;
pub mod error;
pub mod export;
pub mod id;
pub mod import;
pub mod logging;
pub mod repos;
pub mod state;
#[cfg(test)]
pub mod test_utils;
pub mod time;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_os::init())
        .setup(|app| {
            use tauri::Manager;

            let data_dir = app.path().app_data_dir()?;
            crate::logging::install_panic_hook(&data_dir);
            let _ = crate::logging::init(&data_dir);
            log::info!("Albada starting; data dir: {:?}", data_dir);

            let db_path = crate::db::path::database_path(app.handle())?;
            let db = crate::state::Db::open(&db_path)?;
            db.with_tx(crate::repos::block_recurrence_repo::reconcile_active_occurrences)?;
            log::info!("SQLite opened at {:?}", db_path);
            app.manage(db);
            // Export cancellation registry for cooperative cancellation of running jobs.
            app.manage(crate::commands::export::ExportCancellationState::new());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::health::greet,
            commands::frontend_error::report_frontend_error,
            commands::frontend_startup::report_frontend_startup,
            commands::notifications::get_due_notifications,
            commands::notifications::record_notification_delivery,
            commands::notifications::show_notification,
            commands::reminder_rules::list_reminder_rules,
            commands::reminder_rules::update_reminder_rule,
            commands::notification_deliveries::list_notification_deliveries,
            commands::notification_deliveries::dismiss_notification,
            commands::notification_deliveries::snooze_notification,
            commands::notification_deliveries::clear_notification_history,
            commands::workspace::create_workspace,
            commands::workspace::get_workspace,
            commands::workspace::list_workspaces,
            commands::workspace::update_workspace,
            commands::workspace::update_workspace_locale,
            commands::workspace::ensure_default_workspace,
            commands::workspace::get_warning_window,
            commands::workspace::update_warning_window,
            commands::workspace::get_backup_reminder_preference,
            commands::workspace::update_backup_reminder_preference,
            commands::year_theme::create_year_theme,
            commands::year_theme::get_year_theme,
            commands::year_theme::list_year_themes,
            commands::year_theme::update_year_theme,
            commands::year_theme::archive_year_theme,
            commands::quarter::create_quarter,
            commands::quarter::get_quarter,
            commands::quarter::list_quarters,
            commands::quarter::update_quarter,
            commands::quarter_focus::create_quarter_focus,
            commands::quarter_focus::get_quarter_focus,
            commands::quarter_focus::list_quarter_focus,
            commands::quarter_focus::update_quarter_focus,
            commands::goal::create_goal,
            commands::goal::get_goal,
            commands::goal::list_goals,
            commands::goal::update_goal,
            commands::goal::archive_goal,
            commands::project::create_project,
            commands::project::get_project,
            commands::project::list_projects,
            commands::project::list_projects_for_goal,
            commands::project::update_project,
            commands::project::archive_project,
            commands::project::move_project,
            commands::task::create_task,
            commands::task::create_child_task,
            commands::task::get_task,
            commands::task::get_task_with_tree,
            commands::task::list_tasks,
            commands::task::list_tasks_for_project,
            commands::task::list_tasks_for_goal,
            commands::task::list_standalone_tasks,
            commands::task::list_archived_tasks,
            commands::task::update_task,
            commands::task::move_task,
            commands::task::attach_task,
            commands::task::transition_task_status,
            commands::task::complete_task_cmd,
            commands::task::close_parent_manually,
            commands::task::archive_task,
            commands::task::restore_task,
            commands::task::restore_archived_task,
            commands::task::delete_task,
            commands::task::list_task_history,
            commands::task::why_is_blocked,
            commands::progress::get_project_progress,
            commands::progress::get_goal_progress,
            commands::progress::get_task_progress,
            commands::deadline::get_task_deadline_status,
            commands::deadline::get_project_deadline_status,
            commands::deadline::get_goal_deadline_status,
            commands::deadline::list_at_risk_tasks,
            commands::dependency::add_dependency,
            commands::dependency::remove_dependency,
            commands::dependency::list_blocking_tasks,
            commands::dependency::list_all_prerequisites,
            commands::dependency::list_dependencies_for_task,
            commands::daily_plan::get_or_create_daily_plan,
            commands::daily_plan::get_daily_plan_by_date,
            commands::daily_plan::list_daily_plans_between,
            commands::daily_plan::update_daily_plan_status,
            commands::daily_plan::update_daily_plan_intention,
            commands::daily_plan::update_daily_plan_review,
            commands::daily_plan::set_daily_plan_planning_block,
            commands::daily_plan::select_task_for_day,
            commands::daily_plan::deselect_task_for_day,
            commands::daily_plan::reorder_day_tasks,
            commands::daily_plan::list_day_tasks,
            commands::daily_plan::replan_task,
            commands::daily_plan::get_capacity_suggestion,
            commands::block::create_block,
            commands::block::update_block,
            commands::block::create_block_recurrence,
            commands::block::update_block_recurrence,
            commands::block::get_block_recurrence,
            commands::block::get_block,
            commands::block::start_block_cmd,
            commands::block::complete_block_cmd,
            commands::block::skip_block_cmd,
            commands::block::cancel_block_cmd,
            commands::block::list_blocks_for_date,
            commands::block::list_blocks_for_plan,
            commands::block::list_blocks_between,
            commands::block::deactivate_block_cmd,
            commands::block::reactivate_block_cmd,
            commands::block::delete_block_cmd,
            commands::block::deactivate_block_recurrence_cmd,
            commands::block::reactivate_block_recurrence_cmd,
            commands::block::delete_block_recurrence_cmd,
            commands::block::list_inactive_blocks_cmd,
            commands::block::list_inactive_block_recurrences_cmd,
            commands::block::list_scheduled_block_recurrences_cmd,
            commands::block::list_upcoming_blocks_cmd,
            commands::block::get_block_recurrence_analytics_cmd,
            commands::session::create_session,
            commands::session::get_session,
            commands::session::start_session_cmd,
            commands::session::pause_session_cmd,
            commands::session::finish_session_cmd,
            commands::session::skip_session_cmd,
            commands::session::list_sessions_for_task,
            commands::session::list_sessions_for_block,
            commands::session::list_sessions_for_day_task,
            commands::fixed_commitment::create_fixed_commitment,
            commands::fixed_commitment::get_fixed_commitment,
            commands::fixed_commitment::list_fixed_commitments,
            commands::fixed_commitment::update_fixed_commitment,
            commands::fixed_commitment::set_fixed_commitment_active,
            commands::fixed_commitment::delete_fixed_commitment,
            commands::availability_profile::create_availability_profile,
            commands::availability_profile::get_availability_profile,
            commands::availability_profile::list_availability_profiles,
            commands::availability_profile::update_availability_profile,
            commands::availability_profile::delete_availability_profile,
            commands::journal::create_journal_entry,
            commands::journal::get_journal_entry,
            commands::journal::update_journal_entry,
            commands::journal::list_journal_entries,
            commands::journal::archive_journal_entry,
            commands::journal::archive_journal_entries,
            commands::journal::set_journal_entries_export_eligible,
            commands::journal_reference::add_journal_reference,
            commands::journal_reference::remove_journal_reference,
            commands::journal_reference::list_references_for_entry,
            commands::markdown::add_word_to_native_dictionary,
            commands::export::create_export_job,
            commands::export::list_export_jobs,
            commands::export::delete_export_job,
            commands::export::run_export_job,
            commands::export::cancel_export_job,
            commands::export::preview_export_selection,
            commands::export::export_template_capability,
            commands::export::export_template_catalog,
            commands::export::save_export_artifact_copy,
            commands::diagnostics::get_environment_diagnostics,
            commands::therapy::create_therapy_collection,
            commands::therapy::list_therapy_collections,
            commands::therapy::update_therapy_collection,
            commands::therapy::archive_therapy_collection,
            commands::therapy::create_therapy_record,
            commands::therapy::get_therapy_record,
            commands::therapy::update_therapy_record,
            commands::therapy::list_therapy_records,
            commands::therapy::archive_therapy_record,
            commands::therapy_reference::add_therapy_reference,
            commands::therapy_reference::remove_therapy_reference,
            commands::therapy_reference::list_references_for_record,
            commands::import::run_legacy_import,
            commands::import::start_legacy_import,
            commands::import::preview_legacy_import,
            commands::import::commit_legacy_import,
            commands::import::get_import_batch,
            commands::import::get_import_batch_status,
            commands::backup::create_backup,
            commands::backup::list_backups,
            commands::backup::delete_backup,
            commands::backup::validate_backup,
            commands::backup::restore_backup,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| log::error!("Tauri application stopped: {error}"));
}

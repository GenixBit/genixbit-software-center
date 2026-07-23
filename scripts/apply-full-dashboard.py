from pathlib import Path


def replace_once(text: str, old: str, new: str, path: Path) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one marker, found {count}: {old[:120]!r}")
    return text.replace(old, new, 1)


path = Path("crates/software-center/src/main.rs")
text = path.read_text()
text = replace_once(text, "mod client;\n", "mod client;\nmod dashboard_view;\n", path)
text = replace_once(
    text,
    "use activity_time::{current_unix_ms, timing_text};\n",
    "use activity_time::{current_unix_ms, timing_text};\nuse dashboard_view::summarize_dashboard;\n",
    path,
)
text = replace_once(
    text,
    "    health_list: gtk::ListBox,\n",
    "    health_list: gtk::ListBox,\n    dashboard_health: Rc<RefCell<Option<SystemHealth>>>,\n",
    path,
)
text = replace_once(
    text,
    "        health_list,\n        installed_entry,\n",
    "        health_list,\n        dashboard_health: Rc::new(RefCell::new(None)),\n        installed_entry,\n",
    path,
)
text = replace_once(
    text,
    "    populate_installed_sections(ui);\n    render_health(ui, &snapshot.health);\n",
    "    populate_installed_sections(ui);\n    *ui.dashboard_health.borrow_mut() = Some(snapshot.health);\n    render_dashboard(ui);\n",
    path,
)
text = replace_once(
    text,
    "fn render_health(ui: &UiState, health: &SystemHealth) {\n    clear_list(&ui.health_list);\n    let state = if health.broken_package_count == 0 {\n        \"Package database reports no interrupted states\"\n    } else {\n        \"Package database requires attention\"\n    };\n    ui.dashboard_status.set_text(&format!(\n        \"{state}. {} installed packages, {} available updates and {} security updates.\",\n        health.installed_count, health.update_count, health.security_update_count\n    ));\n",
    "fn render_dashboard(ui: &UiState) {\n    clear_list(&ui.health_list);\n    let health_guard = ui.dashboard_health.borrow();\n    let Some(health) = health_guard.as_ref() else {\n        ui.dashboard_status.set_text(\"Loading system overview…\");\n        return;\n    };\n    let services = ui.services.borrow();\n    let transactions = ui.activity_records.borrow();\n    let summary = summarize_dashboard(health, &services, &transactions);\n    ui.dashboard_status.set_text(&summary.status_text());\n",
    path,
)
text = replace_once(
    text,
    "    append_health_row(\n        &ui.health_list,\n        \"Update origins\",\n        &update_origins,\n        &format!(\"{} sources\", health.update_sources.len()),\n    );\n}\n",
    "    append_health_row(\n        &ui.health_list,\n        \"Update origins\",\n        &update_origins,\n        &format!(\"{} sources\", health.update_sources.len()),\n    );\n    append_health_row(\n        &ui.health_list,\n        \"Approved services\",\n        &format!(\n            \"{} active and {} failed across {} allowlisted services\",\n            summary.active_services, summary.failed_services, summary.approved_services\n        ),\n        if summary.failed_services == 0 { \"Healthy\" } else { \"Attention\" },\n    );\n    append_health_row(\n        &ui.health_list,\n        \"Transaction activity\",\n        &format!(\n            \"{} active, {} failed and {} interrupted across {} recent records\",\n            summary.active_transactions,\n            summary.failed_transactions,\n            summary.interrupted_transactions,\n            summary.recent_transactions\n        ),\n        \"Simulation only\",\n    );\n    append_health_row(\n        &ui.health_list,\n        \"Security posture\",\n        &format!(\n            \"{} security updates and {} broken package states\",\n            summary.security_updates, summary.broken_packages\n        ),\n        if summary.security_updates == 0 && summary.broken_packages == 0 {\n            \"Current\"\n        } else {\n            \"Review\"\n        },\n    );\n}\n",
    path,
)
text = replace_once(
    text,
    "                *ui.services.borrow_mut() = services;\n                render_services(&ui);\n",
    "                *ui.services.borrow_mut() = services;\n                render_services(&ui);\n                render_dashboard(&ui);\n",
    path,
)
text = replace_once(
    text,
    "                *ui.activity_records.borrow_mut() = records;\n                render_activity(&ui);\n",
    "                *ui.activity_records.borrow_mut() = records;\n                render_activity(&ui);\n                render_dashboard(&ui);\n",
    path,
)
text = replace_once(
    text,
    "    clear_list(&ui.health_list);\n    ui.dashboard_status\n",
    "    clear_list(&ui.health_list);\n    ui.dashboard_health.borrow_mut().take();\n    ui.dashboard_status\n",
    path,
)
path.write_text(text)

roadmap_path = Path("docs/ROADMAP.md")
roadmap = roadmap_path.read_text()
roadmap = replace_once(roadmap, "- [ ] Full dashboard\n", "- [x] Full dashboard\n", roadmap_path)
roadmap_path.write_text(roadmap)

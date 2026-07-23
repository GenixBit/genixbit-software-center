from pathlib import Path


def replace_once(text: str, old: str, new: str, path: Path) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one marker, found {count}: {old[:100]!r}")
    return text.replace(old, new, 1)


main_path = Path("crates/genixpkgd/src/main.rs")
main = main_path.read_text()
main = replace_once(
    main,
    "mod simulation_control;\nmod transaction;\n",
    "mod simulation_control;\nmod systemd;\nmod transaction;\n",
    main_path,
)
main = replace_once(
    main,
    "    SystemSnapshot, TransactionEvent, TransactionPreview, TransactionQueueSnapshot,\n    TransactionRecord, UpdateRecord,\n",
    "    ServiceRecord, SystemSnapshot, TransactionEvent, TransactionPreview,\n    TransactionQueueSnapshot, TransactionRecord, UpdateRecord,\n",
    main_path,
)
main = replace_once(
    main,
    "    async fn check_updates(&self) -> zbus::fdo::Result<Vec<UpdateRecord>> {\n        apt::check_updates().await.map_err(dbus_failed)\n    }\n\n",
    "    async fn check_updates(&self) -> zbus::fdo::Result<Vec<UpdateRecord>> {\n        apt::check_updates().await.map_err(dbus_failed)\n    }\n\n    async fn list_approved_services(&self) -> zbus::fdo::Result<Vec<ServiceRecord>> {\n        systemd::inspect_approved_services().await.map_err(dbus_failed)\n    }\n\n",
    main_path,
)
main_path.write_text(main)

xml_path = Path("dbus/com.genixbit.PackageManager1.xml")
xml = xml_path.read_text()
xml = replace_once(
    xml,
    "    <method name=\"CheckUpdates\">\n      <arg name=\"updates\" type=\"a(sssssb)\" direction=\"out\"/>\n    </method>\n",
    "    <method name=\"CheckUpdates\">\n      <arg name=\"updates\" type=\"a(sssssb)\" direction=\"out\"/>\n    </method>\n    <method name=\"ListApprovedServices\">\n      <arg name=\"services\" type=\"a(ssssss)\" direction=\"out\"/>\n    </method>\n",
    xml_path,
)
xml_path.write_text(xml)

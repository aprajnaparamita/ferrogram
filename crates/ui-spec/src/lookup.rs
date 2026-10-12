//! The [`CommandLookup`] builder and the [`LayoutSet`] helpers.
//!
//! [`CommandLookup::build`] walks a [`Layout`] once and records, for
//! every control, its label and the `CommandId` it dispatches, plus
//! the reverse index from `CommandId` to the Fluent ids that surface
//! it. [`LayoutSet::command_lookup`] wraps the builder and
//! [`LayoutSet::select`] is the runtime layout switch.

use std::collections::{BTreeMap, BTreeSet};

use crate::{CommandId, CommandLookup, Control, FluentId, Layout, LayoutKind, LayoutSet};

/// Iterates every control of the layout exactly once, in a fixed
/// order: tab groups (tab order, group order, control order), then
/// backstage tabs, then context menus, then the quick access toolbar.
pub(crate) fn all_controls(layout: &Layout) -> impl Iterator<Item = &Control> {
    let tab_controls = layout
        .tabs
        .iter()
        .flat_map(|tab| tab.groups.iter())
        .flat_map(|group| group.controls.iter());
    let backstage_controls = layout
        .backstage
        .tabs
        .iter()
        .flat_map(|tab| tab.controls.iter());
    let context_menu_controls = layout
        .context_menus
        .iter()
        .flat_map(|menu| menu.controls.iter());
    tab_controls
        .chain(backstage_controls)
        .chain(context_menu_controls)
        .chain(layout.quick_access_toolbar.controls.iter())
}

impl CommandLookup {
    /// Builds the lookup from a layout.
    ///
    /// Walks the layout once, recording for each control its label and
    /// its `command` link, and reverse-indexing each `CommandId` to
    /// the Fluent ids that surface it.
    ///
    /// - `labels` gets one entry for every tab, group, backstage tab,
    ///   context menu and control in the layout. A control's
    ///   [`aliases`](crate::Control::aliases) are recorded with the
    ///   same label, so any Fluent name resolves.
    /// - `commands` maps each control's primary id to its dispatched
    ///   `CommandId`. When a control has aliases the primary path (the
    ///   control's own id) is used; the aliases are not separate keys.
    /// - `fluent_by_command` is the exact reverse of `commands`, with
    ///   sorted, de-duplicated `FluentId` lists.
    pub fn build(layout: &Layout) -> Self {
        let mut labels: BTreeMap<FluentId, String> = BTreeMap::new();
        let mut commands: BTreeMap<FluentId, CommandId> = BTreeMap::new();
        let mut reverse: BTreeMap<CommandId, BTreeSet<FluentId>> = BTreeMap::new();

        for tab in &layout.tabs {
            labels.insert(tab.id.clone(), tab.label.clone());
            for group in &tab.groups {
                labels.insert(group.id.clone(), group.label.clone());
            }
        }
        for tab in &layout.backstage.tabs {
            labels.insert(tab.id.clone(), tab.label.clone());
        }
        for menu in &layout.context_menus {
            labels.insert(menu.id.clone(), menu.label.clone());
        }
        for control in all_controls(layout) {
            labels.insert(control.id.clone(), control.label.clone());
            for alias in &control.aliases {
                labels.insert(alias.clone(), control.label.clone());
            }
            if let Some(command) = &control.command {
                commands.insert(control.id.clone(), command.clone());
                reverse
                    .entry(command.clone())
                    .or_default()
                    .insert(control.id.clone());
            }
        }

        CommandLookup {
            labels,
            commands,
            fluent_by_command: reverse
                .into_iter()
                .map(|(command, ids)| (command, ids.into_iter().collect()))
                .collect(),
        }
    }
}

impl LayoutSet {
    /// Builds the [`CommandLookup`] for this set's layout.
    pub fn command_lookup(&self) -> CommandLookup {
        CommandLookup::build(&self.layout)
    }

    /// Picks the active layout at runtime ("layout switch").
    ///
    /// `visio_2016` and `visio_365` are the loaded sets for the two
    /// Phase 1 layouts; `kind` selects which one is active.
    pub fn select<'a>(
        kind: LayoutKind,
        visio_2016: &'a LayoutSet,
        visio_365: &'a LayoutSet,
    ) -> &'a LayoutSet {
        match kind {
            LayoutKind::Visio2016 => visio_2016,
            LayoutKind::Visio365 => visio_365,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Backstage, BackstageTab, ContextMenu, Group, QuickAccessToolbar, Tab};

    fn control(id: &str, label: &str, command: Option<&str>) -> Control {
        Control {
            id: FluentId::new(id),
            r#type: crate::ControlType::Button,
            label: label.to_owned(),
            order: 0,
            parent: None,
            command: command.map(CommandId::new),
            aliases: Vec::new(),
            bounds: None,
            icon_ref: None,
            keytip: None,
            size_variant: None,
            visual_notes: None,
        }
    }

    fn fixture_layout() -> Layout {
        Layout {
            tabs: vec![Tab {
                id: FluentId::new("TabHome"),
                label: "Home".to_owned(),
                order: 0,
                tab_set: Some("TabSetPrimary".to_owned()),
                bounds: None,
                icon_ref: None,
                keytip: None,
                size_variant: None,
                visual_notes: None,
                groups: vec![Group {
                    id: FluentId::new("GroupClipboard"),
                    label: "Clipboard".to_owned(),
                    order: 0,
                    bounds: None,
                    icon_ref: None,
                    keytip: None,
                    size_variant: None,
                    visual_notes: None,
                    controls: vec![{
                        let mut paste = control("PasteGallery", "Paste", Some("edit.paste"));
                        paste.aliases = vec![FluentId::new("PasteMenu")];
                        paste
                    }],
                }],
            }],
            backstage: Backstage {
                tabs: vec![BackstageTab {
                    id: FluentId::new("FileNew"),
                    label: "New".to_owned(),
                    order: 0,
                    bounds: None,
                    icon_ref: None,
                    keytip: None,
                    size_variant: None,
                    visual_notes: None,
                    controls: vec![control("FileSave", "Save", Some("file.save"))],
                }],
            },
            context_menus: vec![ContextMenu {
                id: FluentId::new("ShapeShortcutMenu"),
                label: "Shape shortcut menu".to_owned(),
                order: 0,
                bounds: None,
                icon_ref: None,
                keytip: None,
                size_variant: None,
                visual_notes: None,
                // A second surface for `file.save`; also a control
                // with no command at all.
                controls: vec![
                    control("FileSaveMenu", "Save", Some("file.save")),
                    control("Cut", "Cut", None),
                ],
            }],
            quick_access_toolbar: QuickAccessToolbar {
                controls: vec![control("UndoQuickAccess", "Undo", Some("edit.undo"))],
            },
            ..Layout::default()
        }
    }

    #[test]
    fn build_records_labels_for_every_surface() {
        let lookup = CommandLookup::build(&fixture_layout());
        assert_eq!(
            lookup.labels.get(&FluentId::new("TabHome")),
            Some(&"Home".to_owned())
        );
        assert_eq!(
            lookup.labels.get(&FluentId::new("GroupClipboard")),
            Some(&"Clipboard".to_owned())
        );
        assert_eq!(
            lookup.labels.get(&FluentId::new("FileNew")),
            Some(&"New".to_owned())
        );
        assert_eq!(
            lookup.labels.get(&FluentId::new("ShapeShortcutMenu")),
            Some(&"Shape shortcut menu".to_owned())
        );
        assert_eq!(
            lookup.labels.get(&FluentId::new("Cut")),
            Some(&"Cut".to_owned())
        );
        assert_eq!(
            lookup.labels.get(&FluentId::new("UndoQuickAccess")),
            Some(&"Undo".to_owned())
        );
    }

    #[test]
    fn build_records_aliases_with_the_control_label() {
        let lookup = CommandLookup::build(&fixture_layout());
        assert_eq!(
            lookup.labels.get(&FluentId::new("PasteMenu")),
            Some(&"Paste".to_owned())
        );
        // The alias is a label entry only: the primary path is used
        // for dispatch.
        assert!(!lookup.commands.contains_key(&FluentId::new("PasteMenu")));
        assert_eq!(
            lookup.commands.get(&FluentId::new("PasteGallery")),
            Some(&CommandId::new("edit.paste"))
        );
    }

    #[test]
    fn build_skips_controls_without_commands() {
        let lookup = CommandLookup::build(&fixture_layout());
        assert!(!lookup.commands.contains_key(&FluentId::new("Cut")));
    }

    #[test]
    fn build_reverse_index_is_sorted_and_deduped() {
        let lookup = CommandLookup::build(&fixture_layout());
        assert_eq!(
            lookup.fluent_by_command.get(&CommandId::new("file.save")),
            Some(&vec![
                FluentId::new("FileSave"),
                FluentId::new("FileSaveMenu")
            ])
        );
        assert_eq!(
            lookup.fluent_by_command.get(&CommandId::new("edit.undo")),
            Some(&vec![FluentId::new("UndoQuickAccess")])
        );
    }

    #[test]
    fn all_controls_walks_every_surface_once() {
        let layout = fixture_layout();
        let ids: Vec<String> = all_controls(&layout).map(|c| c.id.to_string()).collect();
        assert_eq!(
            ids,
            vec![
                "PasteGallery",
                "FileSave",
                "FileSaveMenu",
                "Cut",
                "UndoQuickAccess"
            ]
        );
    }

    #[test]
    fn command_lookup_on_layout_set() {
        let set = LayoutSet {
            layout: fixture_layout(),
            ..LayoutSet::default()
        };
        let lookup = set.command_lookup();
        // Tabs/groups/backstage/menus plus controls and aliases:
        // TabHome, GroupClipboard, FileNew, ShapeShortcutMenu,
        // PasteGallery, PasteMenu, FileSave, FileSaveMenu, Cut,
        // UndoQuickAccess.
        assert_eq!(lookup.len(), 10);
        assert!(!lookup.is_empty());
    }

    #[test]
    fn select_picks_the_active_layout() {
        let mut a = LayoutSet::default();
        a.layout.source = "2016".to_owned();
        let mut b = LayoutSet::default();
        b.layout.source = "365".to_owned();
        assert_eq!(LayoutSet::select(LayoutKind::Visio2016, &a, &b), &a);
        assert_eq!(LayoutSet::select(LayoutKind::Visio365, &a, &b), &b);
    }

    #[test]
    fn empty_layout_builds_empty_lookup() {
        let lookup = CommandLookup::build(&Layout::default());
        assert!(lookup.is_empty());
        assert_eq!(lookup.len(), 0);
    }
}

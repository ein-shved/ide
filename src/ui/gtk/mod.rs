mod grid_cell;

use crate::Project;

use crate::ui::gtk::grid_cell::Entry;
use crate::ui::gtk::grid_cell::GridCell;

use gtk::gio;
use gtk::glib::BoxedAnyObject;
use gtk::prelude::*;
use gtk::{glib, Application};
use gtk4 as gtk;

use std::cell::Ref;
use std::io;
use std::rc::Rc;

pub struct Gtk {
    data: Rc<GtkData>,
}

struct GtkData {
    projects: Vec<Project>,
}

impl super::Ui for Gtk {
    fn run(&self) -> io::Result<()> {
        self.main();
        Ok(())
    }
}

impl Gtk {
    pub fn new<Projects: Iterator<Item = Project>>(projects: Projects) -> Gtk {
        Gtk {
            data: Rc::new(GtkData {
                projects: projects.collect(),
            }),
        }
    }

    fn main(&self) -> glib::ExitCode {
        let application = Application::builder()
            .application_id("com.example.FirstGtkApp")
            .build();

        let data = self.data.clone();
        application.connect_activate(move |application| Gtk::build_ui(data.as_ref(), application));

        application.run()
    }

    fn build_ui(data: &GtkData, application: &gtk::Application) {
        let window = gtk::ApplicationWindow::builder()
            .application(application)
            .title("Projects")
            .build();

        let store = gio::ListStore::new::<BoxedAnyObject>();

        for proj in &data.projects {
            store.append(&BoxedAnyObject::new(proj.clone()))
        }
        let sel = gtk::SingleSelection::new(Some(store));
        let columnview = gtk::ColumnView::new(Some(sel));

        let col1factory = gtk::SignalListItemFactory::new();
        let col2factory = gtk::SignalListItemFactory::new();
        col1factory.connect_setup(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let row = GridCell::new();
            item.set_child(Some(&row));
        });

        col1factory.connect_bind(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let child = item.child().and_downcast::<GridCell>().unwrap();
            let entry = item.item().and_downcast::<BoxedAnyObject>().unwrap();
            let r: Ref<Project> = entry.borrow();
            let ent = Entry {
                name: r.name.clone(),
            };
            child.set_entry(&ent);
        });
        col2factory.connect_setup(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let row = GridCell::new();
            item.set_child(Some(&row));
        });

        col2factory.connect_bind(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let child = item.child().and_downcast::<GridCell>().unwrap();
            let entry = item.item().and_downcast::<BoxedAnyObject>().unwrap();
            let r: Ref<Project> = entry.borrow();
            let ent = Entry {
                name: String::from(r.path.to_str().unwrap()),
            };
            child.set_entry(&ent);
        });
        let col1 = gtk::ColumnViewColumn::new(Some("Project"), Some(col1factory));
        let col2 = gtk::ColumnViewColumn::new(Some("Path"), Some(col2factory));
        columnview.append_column(&col1);
        columnview.append_column(&col2);

        let scrolled_window = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never) // Disable horizontal scrolling
            .build();

        scrolled_window.set_child(Some(&columnview));
        scrolled_window.set_propagate_natural_height(true);
        scrolled_window.set_max_content_height(500);

        window.set_child(Some(&scrolled_window));
        window.set_resizable(false);
        window.present();
    }
}

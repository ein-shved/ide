mod grid_cell;

use crate::Project;

use grid_cell::Entry;
use grid_cell::GridCell;

use gtk4 as gtk;
use gtk::gdk;
use gtk::gio;
use gtk::glib::BoxedAnyObject;
use gtk::prelude::*;
use gtk::{glib, Application};

use std::cell::{Ref, RefCell};
use std::rc::Rc;

use paste;

use super::{Projects, Ui, UiFactory};

pub struct GtkFactory {}

type RcData = Rc<RefCell<GtkData>>;
type RcProjects = Vec<Rc<Project>>;

impl UiFactory for GtkFactory {
    fn new<'a>(&self, projects: Projects<'a>) -> Box<dyn Ui> {
        Box::new(Gtk {
            data: Rc::new(RefCell::new(GtkData {
                projects: projects.map(|proj| Rc::new(proj)).collect(),
                window: None,
            })),
            result: None,
        })
    }
    fn preferred_editor(&self) -> Option<String> {
        Some(String::from("neovide"))
    }
}

impl super::Ui for Gtk {
    fn run(&mut self) -> Option<Project> {
        self.main();
        self.result.clone()
    }
}

pub struct Gtk {
    data: Rc<RefCell<GtkData>>,
    result: Option<Project>,
}

struct GtkData {
    projects: RcProjects,
    window: Option<GtkWindow>,
}

struct GtkWindow {
    data: RcData,
    window: gtk::ApplicationWindow,
    table: gtk::ColumnView,

    cl_names: gtk::ColumnViewColumn,
    cl_paths: gtk::ColumnViewColumn,

    bt_open: gtk::Button,
    bt_new: gtk::Button,
    bt_remove: gtk::Button,
    result: Option<Rc<Project>>,
}

macro_rules! rc2win {
    ( $data:ident ) => {
        $data.borrow_mut().window.as_mut().unwrap()
    };
}

macro_rules! make_button {
    ( $type:ident) => {
        paste::item! {
            fn [< make_bt_ $type >](data: RcData) -> gtk::Button {
                let mut name = stringify!($type).chars();
                let name = match name.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + name.as_str(),
                };
                let btn = gtk::Button::with_label(&name);
                btn.connect_clicked(move |_| rc2win!(data).[< on_ $type >]());
                btn
            }
        }
    };
}

impl GtkWindow {
    fn new(application: &Application, data: RcData) -> GtkWindow {
        let mut window = GtkWindow {
            data: data.clone(),
            window: Self::make_window(application),
            table: Self::make_table(data.clone()),
            cl_names: Self::make_cl_names(),
            cl_paths: Self::make_cl_paths(),
            bt_open: Self::make_bt_open(data.clone()),
            bt_new: Self::make_bt_new(data.clone()),
            bt_remove: Self::make_bt_remove(data.clone()),
            result: None,
        };
        window.construct();
        window
    }

    fn make_window(application: &Application) -> gtk::ApplicationWindow {
        gtk::ApplicationWindow::builder()
            .application(application)
            .title("Projects")
            .build()
    }

    fn make_cl_names() -> gtk::ColumnViewColumn {
        let col1factory = gtk::SignalListItemFactory::new();

        col1factory.connect_setup(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let row = GridCell::new();
            item.set_child(Some(&row));
        });
        col1factory.connect_bind(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let child = item.child().and_downcast::<GridCell>().unwrap();
            let entry = item.item().and_downcast::<BoxedAnyObject>().unwrap();
            let r = entry.borrow::<Rc<Project>>().as_ref().name.clone();
            child.set_min_chars(r.len() as u32);
            let ent = Entry { name: r.clone() };
            child.set_entry(&ent);
        });

        gtk::ColumnViewColumn::new(Some("Project"), Some(col1factory))
    }

    fn make_cl_paths() -> gtk::ColumnViewColumn {
        let col2factory = gtk::SignalListItemFactory::new();

        col2factory.connect_setup(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let row = GridCell::new();
            item.set_child(Some(&row));
        });
        col2factory.connect_bind(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let child = item.child().and_downcast::<GridCell>().unwrap();
            let entry = item.item().and_downcast::<BoxedAnyObject>().unwrap();
            let r: Ref<Rc<Project>> = entry.borrow();
            let r = r.path.to_str().unwrap();
            let ent = Entry {
                name: String::from(r),
            };
            child.set_min_chars(std::cmp::min(r.len(), 100) as u32);
            child.set_entry(&ent);
        });

        gtk::ColumnViewColumn::new(Some("Path"), Some(col2factory))
    }

    fn make_table(data: RcData) -> gtk::ColumnView {
        let projects = &data.borrow().projects;
        let store = gio::ListStore::new::<BoxedAnyObject>();
        for proj in projects {
            store.append(&BoxedAnyObject::new(proj.clone()))
        }

        let sel = gtk::SingleSelection::new(Some(store));
        gtk::ColumnView::new(Some(sel))
    }

    make_button!(open);
    make_button!(new);
    make_button!(remove);

    fn add_controllers(&mut self) {
        let controller = gtk::EventControllerKey::new();
        let data = self.data.clone();
        controller.connect_key_released(move |_, keyval, _, _| {
            rc2win!(data).on_key(keyval)
        });
        self.window.add_controller(controller);

        let data = self.data.clone();
        self.table.connect_activate(move |_, num| {
            rc2win!(data).on_open_at(Some(num));
        });
    }

    fn construct(&mut self) {
        let scrolled_window = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never) // Disable horizontal scrolling
            .build();

        scrolled_window.set_child(Some(&self.table));
        scrolled_window.set_propagate_natural_height(true);
        scrolled_window.set_max_content_height(500);

        let grid = gtk::Grid::builder()
            .margin_start(6)
            .margin_end(6)
            .margin_top(6)
            .margin_bottom(6)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .row_spacing(6)
            .column_spacing(6)
            .build();

        grid.attach(&scrolled_window, 0, 0, 3, 1);

        grid.attach(&self.bt_open, 0, 1, 1, 1);
        grid.attach(&self.bt_new, 1, 1, 1, 1);
        grid.attach(&self.bt_remove, 2, 1, 1, 1);

        self.table.append_column(&self.cl_names);
        self.table.append_column(&self.cl_paths);

        self.add_controllers();
        self.window.set_child(Some(&grid));
        self.window.set_resizable(false);
    }

    pub fn present(&self) {
        self.window.present();
    }

    fn on_new(&mut self) {
        let dialog = gtk::FileDialog::new();
        let cancellable: Option<&gio::Cancellable> = None;
        let data = self.data.clone();
        dialog.select_folder(Some(&self.window), cancellable, move |res| {
            if let Ok(res) = res {
                let res = res.path().unwrap();
                rc2win!(data).do_open(Rc::new(Project::from_path(&res.to_str().unwrap())));
            }
        });
    }

    fn on_open(&mut self) {
        self.on_open_at(None)
    }

    fn on_open_at(&mut self, num: Option<u32>) {
        let mut proj = None;
        if let Some(num) = num {
            proj = self.get(num);
        }
        if proj.is_none() {
            proj = self.get_selected();
        }
        if let Some(proj) = proj {
            self.do_open(proj);
        }
    }

    fn on_remove(&mut self) {
        self.with_selection(|me, selection| {
            me.do_remove(selection.selected());
        });
    }

    fn on_key(&mut self, keyval: gdk::Key) {
        use gdk::Key;
        match keyval {
            Key::Escape => self.on_exit(),
            Key::Return => self.on_open(),
            Key::Delete => self.on_remove(),
            Key::BackSpace => self.on_remove(),
            Key::d => self.on_remove(),
            _ => (),
        }
    }

    fn on_exit(&mut self) {
        self.window.close();
    }

    fn with_selection<F, T>(&mut self, f: F) -> Option<T>
    where
        F: FnOnce(&mut Self, &gtk::SingleSelection) -> T,
    {
        if let Some(model) = self.table.model().as_ref() {
            if let Some(selection) = model.downcast_ref::<gtk::SingleSelection>() {
                return Some(f(self, selection));
            }
        };
        None
    }
    fn get_project_by<F>(&mut self, f: F) -> Option<Rc<Project>>
    where
        F: FnOnce(&mut Self, &gtk::SingleSelection) -> Option<glib::object::Object>
    {
        let proj = self.with_selection(|me, selection| -> Option<Rc<Project>> {
            let item = f(me, selection)?;
            let item = item.downcast::<BoxedAnyObject>().unwrap();
            let entry = item.borrow::<Rc<Project>>();
            Some(entry.clone())
        });
        if let Some(proj) = proj {
            proj
        } else {
            None
        }
    }

    fn get_selected(&mut self) -> Option<Rc<Project>> {
        self.get_project_by(|_, selection| {
            selection.selected_item()
        })
    }

    fn get(&mut self, index: u32) -> Option<Rc<Project>> {
        self.get_project_by(|_, selection| {
            selection.item(index)
        })
    }

    fn do_open(&mut self, project: Rc<Project>) {
        self.result = Some(project);
        self.window.close();
    }
    fn do_remove(&mut self, index: u32) {
        self.with_selection(|_, selection| {
            let model = selection.model();
            let store_ptr = model.and_downcast_ref::<gio::ListStore>();
            let store = store_ptr.unwrap();
            if let Some(item) = selection.selected_item() {
                let item = item.downcast::<BoxedAnyObject>().unwrap();
                let entry = item.borrow::<Rc<Project>>();
                let _ = entry.rm();
                store.remove(index);
            }
        });
    }
}

impl Gtk {
    fn main(&mut self) -> glib::ExitCode {
        let application = Application::builder()
            .application_id("com.example.FirstGtkApp")
            .build();

        let data = self.data.clone();
        application.connect_activate(move |application| Gtk::build_ui(data.clone(), application));

        let res = application.run();

        let data = self.data.clone();

        self.result = data
            .borrow()
            .window
            .as_ref()
            .unwrap()
            .result
            .as_ref()
            .map(|proj| proj.as_ref().clone());
        // Break referencve cicle here
        self.data.borrow_mut().window = None;
        res
    }

    fn build_ui(data: RcData, application: &gtk::Application) {
        let window = GtkWindow::new(application, data.clone());
        window.present();
        data.borrow_mut().window = Some(window);
    }
}

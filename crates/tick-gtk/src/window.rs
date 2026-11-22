use adw::subclass::prelude::*;
use gtk::{gio, glib};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/dev/chengrui/Tick/ui/window.ui")]
    pub struct TickWindow {}

    #[glib::object_subclass]
    impl ObjectSubclass for TickWindow {
        const NAME: &'static str = "TickWindow";
        type Type = super::TickWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for TickWindow {}
    impl WidgetImpl for TickWindow {}
    impl WindowImpl for TickWindow {}
    impl ApplicationWindowImpl for TickWindow {}
    impl AdwApplicationWindowImpl for TickWindow {}
}

glib::wrapper! {
    pub struct TickWindow(ObjectSubclass<imp::TickWindow>)
        @extends adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl TickWindow {
    pub fn new(app: &adw::Application) -> Self {
        glib::Object::builder().property("application", app).build()
    }
}

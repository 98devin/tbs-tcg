
// use imgui::*;

use std::sync::mpsc;

pub type GuiItem = Box<dyn Widget>;

pub type WidgetChannel = mpsc::Sender<GuiItem>;
pub type WidgetChannelAccess = parking_lot::Mutex<WidgetChannel>;

pub trait Widget: Send + Sync {
    fn compose(self: Box<Self>, widgets: &WidgetChannel, ui: &imgui::Ui, lua: &rlua::Lua);
}

pub struct WidgetState {
    sender: mpsc::Sender<GuiItem>,
    receiver: mpsc::Receiver<GuiItem>,
    received: Vec<GuiItem>,
}

impl WidgetState {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            sender,
            receiver,
            received: Vec::new(),
        }
    }

    pub fn add_gui_item(&self, item: GuiItem) {
        self.sender.send(item).unwrap();
    }

    pub fn iter_items(&mut self) -> impl Iterator<Item=GuiItem> + '_ {
        self.received.drain(..)
    }

    pub fn refresh_items(&mut self) {
        self.received.extend(self.receiver.try_iter());
    }

    pub fn make_widget_channel(&self) -> WidgetChannel {
        self.sender.clone()
    }
}



pub struct ImguiDemoWindow {
    window_open: bool,
}

impl ImguiDemoWindow {
    pub fn new() -> Box<Self> {
        Box::new(Self { window_open: true })
    }
}

impl Widget for ImguiDemoWindow {
    fn compose(mut self: Box<Self>, widgets: &WidgetChannel, ui: &imgui::Ui, _lua: &rlua::Lua) {
        ui.show_demo_window(&mut self.window_open);
        
        if self.window_open {
            widgets.send(self).unwrap();
        }
    }
}


pub struct LuaPrintBuffer {
    window_name: imgui::ImString,
    printed_strings: Vec<String>,
    console_buffer: imgui::ImString,
}

impl LuaPrintBuffer {
    pub fn new(name: impl Into<imgui::ImString>) -> Box<Self> {
        Box::new(Self {
            window_name: name.into(),
            printed_strings: Vec::new(),
            console_buffer: imgui::ImString::default(),
        })
    }

    fn exec_lua_buffer(&mut self, lua: &rlua::Lua) {

        // WTF: This way rust doesn't infer too strict
        // a lifetime for the following closures.
        let console_buffer = &self.console_buffer;
        let printed_strings = &mut self.printed_strings;

        match lua.context(|ctx: rlua::Context| {
            let chunk = ctx.load(console_buffer.to_str());
            let globals = ctx.globals();
            ctx.scope(|scope| {
                let print_override = scope.create_function_mut(
                    |ctx, args: rlua::MultiValue| {
                        let mut str_args = Vec::with_capacity(args.len());
                        for arg_val in args {
                            let arg_str = ctx.coerce_string(arg_val)?;
                            let arg_str = arg_str.unwrap_or(ctx.create_string("nil")?);
                            str_args.push(arg_str.to_str()?.to_owned());
                        }
                        printed_strings.push(str_args.join("\t"));
                        Ok(())
                    }
                )?;
                globals.set("print", print_override)?;
                let chunk = chunk.set_environment(globals)?;
                chunk.exec()
            })
        }) {
            Ok(()) => (),
            Err(e) => eprintln!("{}", e),
        }
    }
}

impl Widget for LuaPrintBuffer {
    fn compose(mut self: Box<Self>, widgets: &WidgetChannel, ui: &imgui::Ui, lua: &rlua::Lua) {
        use imgui::*;

        let lua_window = Window::new(&self.window_name)
            .size([640.0, 480.0], Condition::FirstUseEver)
            .begin(&ui); 
        
        if let Some(lua_window) = lua_window {
            
            ui.text(im_str!("should be lua, eventually."));
            ui.input_text_multiline(
                im_str!("Lua Console"),
                &mut self.console_buffer, [640.0, 400.0]
            )
            .allow_tab_input(true)
            .resize_buffer(true)
            .build();

            if ui.button(im_str!("Execute"), [100.0, 20.0]) {
                self.exec_lua_buffer(lua);
            }

            lua_window.end(&ui);
        }


        let out_window = Window::new(im_str!("Console output"))
            .size([200.0, 200.0], Condition::FirstUseEver)
            .begin(&ui);

        if let Some(out_window) = out_window {
            
            if ui.button(im_str!("Clear"), [100.0, 20.0]) {
                self.printed_strings.clear();
            }

            ui.separator();

            for output in &self.printed_strings {
                ui.text(output);
            }

            out_window.end(&ui);
        }

        widgets.send(self).unwrap();
    }
}

#[derive(Clone)]
pub struct CloneWindow {
    id: usize,
    name: imgui::ImString,
    pos: [f32; 2],
}

impl CloneWindow {
    pub fn new(id: usize, pos: [f32; 2]) -> Box<Self> {
        Box::new(Self {
            id, pos, name: imgui::im_str!("Clone Window: #{}", id)
        })
    }
}

impl Widget for CloneWindow {
    fn compose(self: Box<Self>, widgets: &WidgetChannel, ui: &imgui::Ui, _lua: &rlua::Lua) {
        use imgui::*;

        let clone_window = Window::new(&self.name)
            .position(self.pos, Condition::Appearing)
            .size([230.0, 90.0], Condition::Appearing)
            .begin(&ui);

        let mut should_close = false;

        if let Some(clone_window) = clone_window {

            should_close = ui.button(im_str!("Close"), [100.0, 50.0]);

            ui.same_line(110.0);

            if ui.button(im_str!("Split"), [100.0, 50.0]) {
                let pos = ui.window_pos();
                widgets.send(CloneWindow::new(self.id * 2 + 1, pos)).unwrap();
                widgets.send(CloneWindow::new(self.id * 2 + 2,
                    [pos[0] + 25.0, pos[1] + 25.0])).unwrap();
                should_close = true;
            }

            clone_window.end(&ui);
        }

        if !should_close {
            widgets.send(self).unwrap();
        }
    }
}
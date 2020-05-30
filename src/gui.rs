
// use imgui::*;


type GuiItem = Box<dyn GuiWidget>;


pub trait GuiWidget: Send {
    fn compose(self: Box<Self>, widgets: &mut ImguiWidgets, ui: &imgui::Ui, lua: &rlua::Lua);
}


#[derive(Default)]
pub struct ImguiWidgets {
    items: Vec<GuiItem>,
}

impl ImguiWidgets {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_widget(&mut self, widget: impl GuiWidget + 'static) {
        self.items.push(Box::new(widget));
    }

    pub fn add_gui_item(&mut self, item: GuiItem) {
        self.items.push(item);
    }

    pub fn iter_items(&mut self) -> impl Iterator<Item=GuiItem> {
        let num_items = self.items.len();
        let old_items = std::mem::replace(&mut self.items, Vec::with_capacity(num_items));
        old_items.into_iter()
    }
}


pub struct ImguiDemoWindow {
    window_open: bool,
}

impl ImguiDemoWindow {
    pub fn new() -> Self {
        Self { window_open: true }
    }
}

impl GuiWidget for ImguiDemoWindow {
    fn compose(mut self: Box<Self>, widgets: &mut ImguiWidgets, ui: &imgui::Ui, _lua: &rlua::Lua) {
        ui.show_demo_window(&mut self.window_open);
        
        if self.window_open {
            widgets.add_gui_item(self);
        }
    }
}


pub struct LuaPrintBuffer {
    window_name: imgui::ImString,
    printed_strings: Vec<String>,
    console_buffer: imgui::ImString,
}

impl LuaPrintBuffer {
    pub fn new(name: impl Into<imgui::ImString>) -> Self {
        Self {
            window_name: name.into(),
            printed_strings: Vec::new(),
            console_buffer: imgui::ImString::default(),
        }
    }

    fn print_override<'lua>(&mut self, ctx: rlua::Context<'lua>, args: rlua::MultiValue<'lua>) -> rlua::Result<()> {
        let mut str_args = Vec::with_capacity(args.len());
        for arg_val in args {
            let arg_str = ctx.coerce_string(arg_val)?;
            let arg_str = arg_str.unwrap_or(ctx.create_string("nil")?);
            str_args.push(arg_str.to_str()?.to_owned());
        }
        self.printed_strings.push(str_args.join("\t"));
        // println!("[Lua]: {}", &self.printed_strings.last().unwrap());
        Ok(())
    }
}

impl GuiWidget for LuaPrintBuffer {
    fn compose(mut self: Box<Self>, widgets: &mut ImguiWidgets, ui: &imgui::Ui, lua: &rlua::Lua) {
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
                match lua.context(|ctx: rlua::Context| {
                    let buffer = self.console_buffer.clone();
                    let chunk = ctx.load(buffer.to_str());
                    let globals = ctx.globals();
                    ctx.scope(|scope| {
                        let print_override =
                            scope.create_function_mut(|ctx, args| self.print_override(ctx, args))?;
                        globals.set("print", print_override)?;
                        let chunk = chunk.set_environment(globals)?;
                        chunk.exec()
                    })
                }) {
                    Ok(()) => (),
                    Err(e) => eprintln!("{}", e),
                }
            }


            lua_window.end(&ui);
        }


        let out_window = Window::new(im_str!("Console output"))
            .size([200.0, 200.0], Condition::FirstUseEver)
            .begin(&ui);

        if let Some(out_window) = out_window {
            
            for output in &self.printed_strings {
                ui.bullet();
                ui.text(output);
            }

            if ui.button(im_str!("Clear"), [100.0, 20.0]) {
                self.printed_strings.clear();
            }

            out_window.end(&ui);
        }

        widgets.add_gui_item(self);
    }
}
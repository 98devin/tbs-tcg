
use imgui::*;


pub trait Widget {
    fn compose(&mut self, ui: &imgui::Ui, lua: &rlua::Lua);
}


pub struct GuiState {
    demo: ImguiDemoWindow,
    lua_print: LuaPrintBuffer,
}


impl GuiState {
    pub fn new() -> Self {
        Self {
            demo: ImguiDemoWindow { window_open: true },
            lua_print: LuaPrintBuffer::new(im_str!("Hello from lua")),
        }
    }
}

impl Widget for GuiState {
    fn compose(&mut self, ui: &imgui::Ui, lua: &rlua::Lua) {
        self.demo.compose(ui, lua);
        self.lua_print.compose(ui, lua);
    }
}


//
//
// Gui Widget Implementations
//
//


struct ImguiDemoWindow {
    window_open: bool,
}

impl Widget for ImguiDemoWindow {
    fn compose(&mut self, ui: &imgui::Ui, _lua: &rlua::Lua) {
        ui.show_demo_window(&mut self.window_open);
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

    fn exec_lua_buffer(&mut self, lua: &rlua::Lua) {

        // WTF: This way rust doesn't infer too strict
        // a lifetime for the following closures.
        let Self {
            ref console_buffer,
            ref mut printed_strings,
            ..
        } = self;

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
    fn compose(&mut self, ui: &imgui::Ui, lua: &rlua::Lua) {
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
    }
}

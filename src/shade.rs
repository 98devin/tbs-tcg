
use std::collections::hash_map::{self, HashMap};


// This is probably enough...?
const MAX_INCLUDE_DEPTH: usize = 5;


fn load_shader_file(name: &str, include_type: shaderc::IncludeType, containing_file: &str, include_depth: usize)
    -> Result<shaderc::ResolvedInclude, String>
{
    use std::fs::File;
    use std::path::PathBuf;
    use std::io::{BufReader, Read};

    if include_depth > MAX_INCLUDE_DEPTH {
        return Err(format!("#include of: {} in: {} exceeded max include depth ({})", name, containing_file, MAX_INCLUDE_DEPTH));
    }

    let path = match include_type {
        shaderc::IncludeType::Standard => PathBuf::from("assets/shaders/"),
        shaderc::IncludeType::Relative => PathBuf::from(containing_file),
    };

    let mut path = match path.canonicalize() {
        Err(io_error) => return Err(format!("{}", io_error)),
        Ok(path) => path,
    };

    path.push(name);

    let file = match File::open(&path) {
        Err(io_error) => return Err(format!("{}", io_error)),
        Ok(file) => file,
    };

    let mut buf_reader = BufReader::new(file);
    let mut content = String::new();

    match buf_reader.read_to_string(&mut content) {
        Err(_) => return Err(format!("File `{}` did not contain proper utf-8.", path.display())),
        Ok(_n_bytes_read) => (),
    };

    let resolved_name = path.to_string_lossy().into();

    Ok(shaderc::ResolvedInclude {
        content,
        resolved_name,
    })
}


pub struct ShaderCacheEntry {
    // origin_path: std::path::PathBuf,
    // psd: wgpu::ProgrammableStageDescriptor<'static>,
    module: wgpu::ShaderModule,
}


impl ShaderCacheEntry {

    pub fn module(&self) -> &wgpu::ShaderModule {
        &self.module
    }

    pub fn descriptor(&self) -> wgpu::ProgrammableStageDescriptor {
        wgpu::ProgrammableStageDescriptor {
            module: &self.module,
            entry_point: "main",
        }
    }

}



pub struct ShaderCache<'d> {
    device: &'d wgpu::Device,
    compiler: shaderc::Compiler,
    options: shaderc::CompileOptions<'static>,
    cache: HashMap<&'static str, ShaderCacheEntry>,
}


impl<'d> ShaderCache<'d> {

    pub fn new(device: &'d wgpu::Device) -> Self {
        let mut options = shaderc::CompileOptions::new().expect("Failed to set glsl compiler options.");
        options.set_auto_bind_uniforms(false);
        options.set_include_callback(load_shader_file);
        
        if cfg!(debug_assertions) {
            // debug mode, add some more careful options.
            options.set_warnings_as_errors();
            options.set_optimization_level(shaderc::OptimizationLevel::Zero);
        } else {
            // release mode, go all-out.
            options.set_suppress_warnings();
            options.set_optimization_level(shaderc::OptimizationLevel::Performance);
        }

        let compiler = shaderc::Compiler::new().expect("Failed to initialize glsl compiler.");
        let cache    = HashMap::new();

        Self { 
            device, 
            compiler, 
            options, 
            cache, 
        }
    }

    pub fn options(&mut self) -> &mut shaderc::CompileOptions<'static> {
        &mut self.options
    }

    pub fn load(&mut self, name: &'static str) -> &ShaderCacheEntry {        
        let vacant = match self.cache.entry(name) {
            hash_map::Entry::Occupied(o) => return o.into_mut(),
            hash_map::Entry::Vacant(v) => v,
        };

        let resolved_file = load_shader_file(name, shaderc::IncludeType::Standard, "", 0)
            .expect("Failed to load shader resource.");

        let glsl_path  = std::path::Path::new(&resolved_file.resolved_name);
        
        let shader_type = match glsl_path.extension() {
            Some(os_str) if os_str == "frag" => shaderc::ShaderKind::Fragment,
            Some(os_str) if os_str == "vert" => shaderc::ShaderKind::Vertex,
            Some(os_str) if os_str == "comp" => shaderc::ShaderKind::Compute,
            _ => panic!("Unknown or missing shader extension: {}", name),
        };
        
        // TODO: Write out to spirv file so we don't have to always recompile?
        // let spirv_path = glsl_path.with_extension("spv");
        
        let preprocessed = self.compiler.preprocess(
            &resolved_file.content,
            &resolved_file.resolved_name,
            "main",
            Some(&self.options),
        ).expect("Failed to preprocess shader!");

        if preprocessed.get_num_warnings() != 0 {
            eprintln!("{}", preprocessed.get_warning_messages());
        }
        
        let spirv = self.compiler.compile_into_spirv(
            &resolved_file.content,
            shader_type,
            &resolved_file.resolved_name,
            "main",
            Some(&self.options),
        ).expect("Failed to compile shader!");
        
        if spirv.get_num_warnings() != 0 {
            eprintln!("{}", spirv.get_warning_messages());
        }

        let shader_module =
            self.device.create_shader_module(spirv.as_binary());

        vacant.insert(ShaderCacheEntry {
            module: shader_module,
        })
    }

    pub fn try_load(&self, name: &'static str) -> Option<&ShaderCacheEntry> {
        self.cache.get(name)
    }

}
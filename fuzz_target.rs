#![feature(new_uninit)]
#![feature(read_buf)]
#![feature(core_io_borrowed_buf)]
#![no_main]
#[macro_use] extern crate libfuzzer_sys;
use rustc_interface::interface;
use rustc_errors::{DiagCtxt, Diagnostic};
use rustc_errors::fallback_fluent_bundle;
use rustc_span::{
    source_map::{FilePathMapping, SourceMap}};
use rustc_data_structures::sync::{Lrc};
use rustc_errors::emitter::{Emitter};
use rustc_errors::translation::Translate;
use rustc_errors::LazyFallbackBundle;
use rustc_driver::DEFAULT_LOCALE_RESOURCES;

/// rustc_driver::Callbacks object that stops before codegen.
pub struct FuzzCallbacks;

/// Emitter which discards every error.
struct SilentEmitter{fb: LazyFallbackBundle}

impl Translate for SilentEmitter {
    fn fluent_bundle(&self) -> Option<&Lrc<rustc_errors::FluentBundle>> {
        None
    }

    fn fallback_fluent_bundle(&self) -> &rustc_errors::FluentBundle {
        &self.fb
    }
}

impl Emitter for SilentEmitter {
    fn source_map(&self) -> Option<&Lrc<SourceMap>> {
        None
    }

    fn emit_diagnostic(&mut self, _db: &Diagnostic) {}
}

impl rustc_driver::Callbacks for FuzzCallbacks {
    fn config(&mut self, config: &mut interface::Config) {

        config.parse_sess_created = Some(Box::new(move |parse_sess| {
         let fallback_bundle =
             rustc_errors::fallback_fluent_bundle(DEFAULT_LOCALE_RESOURCES.to_vec(), false);
            parse_sess.dcx = DiagCtxt::with_emitter(Box::new(SilentEmitter {fb: fallback_bundle}))
        }));
    }
    fn after_analysis<'tcx>(&mut self,
                            _compiler: &rustc_interface::interface::Compiler,
                            _queries: &'tcx rustc_interface::Queries<'tcx>,) -> rustc_driver::Compilation {
        // Stop before codegen.
        rustc_driver::Compilation::Stop
    }
}

/// FileLoader that holds the contents of a single input file in memory.
/// The idea here is to avoid needing to write to disk.
struct FuzzFileLoader {
    // The contents of the single input file.
    input: Vec<u8>,
}

impl FuzzFileLoader {
    fn new(input: Vec<u8>) -> Self {
        FuzzFileLoader {
            input,
        }
    }
}

// The name of the single input file.
const INPUT_PATH: &str = "fuzz_input.rs";

impl rustc_span::source_map::FileLoader for FuzzFileLoader {
    fn file_exists(&self, path: &std::path::Path) -> bool {
        std::path::Path::new(INPUT_PATH) == path
    }

    fn read_file(&self, path: &std::path::Path) -> std::io::Result<String> {
        if self.file_exists(path) {
            let s = match String::from_utf8(self.input.to_vec()) {
                Ok(s) => s,
                Err(_e) => return Err(std::io::Error::new(std::io::ErrorKind::Other,
                                                          "not utf8")),
            };
            Ok(s)
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "tried to open nonexistent file".to_string()))
        }
    }

    fn read_binary_file(&self, path: &std::path::Path) -> std::io::Result<rustc_data_structures::sync::Lrc<[u8]>> {
        use rustc_data_structures::sync::Lrc;
        use std::io::Read;
        if self.file_exists(path) {
            let len = self.input.len();
            let mut bytes = Lrc::new_uninit_slice(len as usize);
            let mut buf = std::io::BorrowedBuf::from(Lrc::get_mut(&mut bytes).unwrap());
            let mut file = std::io::Cursor::new(&self.input[..]);
            file.read_buf_exact(buf.unfilled())?;
            Ok(unsafe { bytes.assume_init() })
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "tried to open nonexistent file".to_string()))
        }
    }
}

/// CodegenBackend that panics when being called to do any actual codegen.
/// We use this to avoid needing to compile rustc_codegen_llvm.
pub struct NullCodegenBackend;

impl rustc_codegen_ssa::traits::CodegenBackend for NullCodegenBackend {
    fn codegen_crate<'tcx>(&self,
                           _: rustc_middle::ty::TyCtxt<'tcx>,
                           _: rustc_metadata::EncodedMetadata,
                           _: bool) -> std::boxed::Box<(dyn core::any::Any + 'static)> {
        unimplemented!()
    }

    fn join_codegen(
        &self,
        _ongoing_codegen: Box<dyn core::any::Any>,
        _sess: &rustc_session::Session,
        _outputs: &rustc_session::config::OutputFilenames,
    ) -> Result<(rustc_codegen_ssa::CodegenResults,
                 rustc_data_structures::fx::FxIndexMap<rustc_middle::dep_graph::WorkProductId,
                                                      rustc_middle::dep_graph::WorkProduct>),
                rustc_errors::ErrorGuaranteed> {
        unimplemented!()
    }

    fn link(
        &self,
        _sess: &rustc_session::Session,
        _codegen_results: rustc_codegen_ssa::CodegenResults,
        _outputs: &rustc_session::config::OutputFilenames,
    ) -> Result<(), rustc_errors::ErrorGuaranteed> {
        unimplemented!()
    }

    fn locale_resource(&self) -> &'static str {
        ""
    }
}


pub fn main_fuzz(input: Vec<u8>) {
    let file_loader = Box::new(FuzzFileLoader::new(input));
    let mut callbacks = FuzzCallbacks;
    let _result = rustc_driver::catch_fatal_errors(|| {
        let args = &["rustc".to_string(),
              INPUT_PATH.to_string(),
              "-o".to_string(),
              "dummy_output_file".to_string(),
              "--edition".to_string(),
              "2021".to_string(),
              "-L".to_string(),
              env!("FUZZ_RUSTC_LIBRARY_DIR").to_string(),
        ];
        let mut run_compiler = rustc_driver::RunCompiler::new(args, &mut callbacks);
        run_compiler.set_file_loader(Some(file_loader));
        run_compiler.set_make_codegen_backend(
            Some(Box::new(|_| {Box::new(NullCodegenBackend)})));
        run_compiler.run()
    }).and_then(|result| result);
}

fuzz_target!(|data: &[u8]| {
    if data.contains(&0x0c) || data.contains(&0x0d) || data.contains(&0x0b) /*|| data.contains (&b'&')*/ {
        return;
    }
//    if let Ok(t) = std::str::from_utf8(data) {
//        if let Some(_) = t.find("derive") {
//            return;
//        }
        main_fuzz(data.into());
//    }
});

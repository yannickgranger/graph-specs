use crate::provenance::{find_owned_unit, module_path_of};
use crate::walk::parse_text;
use ports::{CodeFileSet, ReaderError};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use syn::File;

struct Entry {
    parsed: File,
    unit: Option<String>,
    module_path: Option<String>,
}

#[derive(Clone)]
pub struct ParseCache {
    entries: Rc<RefCell<HashMap<PathBuf, Rc<Entry>>>>,
    order: Rc<Vec<PathBuf>>,
}

impl ParseCache {
    pub fn for_each(&self, mut visit: impl FnMut(&Path, &File, Option<&str>, Option<&str>)) {
        let entries = self.entries.borrow();
        for path in self.order.iter() {
            if let Some(entry) = entries.get(path) {
                visit(
                    path,
                    &entry.parsed,
                    entry.unit.as_deref(),
                    entry.module_path.as_deref(),
                );
            }
        }
    }

    #[must_use]
    pub fn paths(&self) -> &[PathBuf] {
        &self.order
    }
}

pub fn parse(root: &Path, code_set: &CodeFileSet) -> Result<ParseCache, ReaderError> {
    let mut entries = HashMap::new();
    let mut order = Vec::new();
    for file in code_set.files() {
        let parsed = parse_text(&file.text, &file.path)?;
        let unit = find_owned_unit(&file.path, root);
        let module_path = module_path_of(&file.path, root, unit.as_deref());
        entries.insert(
            file.path.clone(),
            Rc::new(Entry {
                parsed,
                unit,
                module_path,
            }),
        );
        order.push(file.path.clone());
    }
    Ok(ParseCache {
        entries: Rc::new(RefCell::new(entries)),
        order: Rc::new(order),
    })
}

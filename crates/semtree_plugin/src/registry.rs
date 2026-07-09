use crate::traits::{FormatterPlugin, LanguagePlugin, LintRulePlugin, LinterPlugin, QueryPlugin};

pub struct PluginRegistry {
    languages: Vec<Box<dyn LanguagePlugin>>,
    linters: Vec<Box<dyn LinterPlugin>>,
    formatters: Vec<Box<dyn FormatterPlugin>>,
    queries: Vec<Box<dyn QueryPlugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            languages: Vec::new(),
            linters: Vec::new(),
            formatters: Vec::new(),
            queries: Vec::new(),
        }
    }

    pub fn register_language(&mut self, plugin: Box<dyn LanguagePlugin>) {
        self.languages.push(plugin);
    }

    pub fn register_linter(&mut self, plugin: Box<dyn LinterPlugin>) {
        self.linters.push(plugin);
    }

    pub fn register_formatter(&mut self, plugin: Box<dyn FormatterPlugin>) {
        self.formatters.push(plugin);
    }

    pub fn register_query(&mut self, plugin: Box<dyn QueryPlugin>) {
        self.queries.push(plugin);
    }

    pub fn get_language_for_extension(&self, ext: &str) -> Option<&dyn LanguagePlugin> {
        self.languages
            .iter()
            .find(|lang| lang.file_extensions().contains(&ext))
            .map(|b| b.as_ref())
    }

    pub fn get_all_lint_rules(&self) -> Vec<Box<dyn LintRulePlugin>> {
        let mut rules: Vec<Box<dyn LintRulePlugin>> = Vec::new();
        for linter in &self.linters {
            rules.extend(linter.rules());
        }
        rules
    }

    pub fn languages(&self) -> &[Box<dyn LanguagePlugin>] {
        &self.languages
    }

    pub fn linters(&self) -> &[Box<dyn LinterPlugin>] {
        &self.linters
    }

    pub fn formatters(&self) -> &[Box<dyn FormatterPlugin>] {
        &self.formatters
    }

    pub fn query_plugins(&self) -> &[Box<dyn QueryPlugin>] {
        &self.queries
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

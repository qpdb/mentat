initSidebarItems({"enum":[["VocabularyCheck","This enum captures the various relationships between a particular vocabulary pair — one `Definition` and one `Vocabulary`, if present."],["VocabularyOutcome","This enum captures the outcome of attempting to ensure that a vocabulary definition is present and up-to-date in the store."]],"struct":[["AttributeBuilder",""],["Definition","A definition of an attribute that is independent of a particular store."],["SimpleVocabularySource","A convenience struct to package simple `pre` and `post` functions with a collection of vocabulary `Definition`s."],["Vocabularies","A collection of named `Vocabulary` instances, as retrieved from the store."],["Vocabulary","A definition of a vocabulary as retrieved from a particular store."]],"trait":[["HasVocabularies","This trait captures the ability to retrieve and describe stored vocabularies."],["VersionedStore","This trait captures the ability of a store to check and install/upgrade vocabularies."],["VocabularySource","Implement `VocabularySource` to have full programmatic control over how a set of `Definition`s are checked against and transacted into a store."],["VocabularyStatus","`VocabularyStatus` is passed to `pre` function when attempting to add or upgrade vocabularies via `ensure_vocabularies`. This is how you can find the status and versions of existing vocabularies — you can retrieve the requested definition and the resulting `VocabularyCheck` by name."]],"type":[["Datom",""],["Version",""]]});
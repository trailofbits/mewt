
## Related to addition of Dialects

- do we not need per-dialect syntax for js? It'd mirror move more closely if we did
- config_for_dialect is inconsistent between JS & Move
- looks like we can rm config_for_target_path from js dialect?
- revisit `pub fn` deltas between js & move dialect mods, they should prob mirror each other more closely
- should engine_for_dialect be on the LanguageResolver trait? What about other stuff on MoveLanguageResolver or JavascriptLanguageResolver? Or are those just internal impl details & nbd?
- rm lang name from stuff exported by dialect mods, eg s/is_<lang>_language_name/is_language_name/ , i think scoping will let us do this & keep things unambiguous
- why no accepts_cli_dialect on js resolver?
- rm `dialect_default.defaulted` and just always log the lang/dialect as part of pretty target printing


## Languages

- review C++ specific mutations, can we re-use common patterns better? Or move those custom mutation patterns out of the engine file?
- revisit high-sev C++ mutations, these could maybe be medium severity?
- resolver.is_language_name and resolver.resolve both have lists of strings like "cpp", "cc", "c++" but these lists are different, should they be shared?
- rename s/JavaScriptDialectEngine/JavascriptEngine/ maybe?
- consolidate MoveDialectConfig and JavascriptDialectConfig into one common DialectConfig trait/struct
- add solidity resolver tests that mirror rusts'

## Types

- combine stuff like resolve_test_cmd + timeout into just resolve_test for that whole config subsection?
- revisit config.resolve_language_defaults
- fix naming of is_path_excluded vs path_is_included
- rename TestFail outcome to Caught
- rm CampaignSummary impl piece? And move it into the stats.rs file?
- comment references move in types/target , and the actual move resolution is not clear, how does it work?
- target uniqueness check; reconsider keying target uniqueness on hash

## Engine

- shuffle_impl pattern is weird, it's basically two fns in one that switches on a bool flag, consider dividing it into two fns
- move internal pattern helpers to utils? Or not worth it?

## Cmds

- is there some print-to-toml lib we can use instead of the bespoke config printer? Or not as pretty as what we have?
- print mutant: also print target/mutant metadata?
- mutants printer format should prob be an enum instead of raw string
- the store arg given to print prob shouldn't be optional, that should always be available iiuc
- purge confirmation message is garbled, clarify the outcomes took the given amount of time
- do we need to be able to print just mutant ids from the results printer? Isn't that more of a mutations printer responsibility?
- why does run generate new mutants if resolved target is given? when/why would this trigger?
- add a --force flag to purge to skip confirmations

## Core
- language_from_path should prob be called engine_from_path
- store.add_mutant should prob return the id instead of None like add_target
- match_target_ids should get moved out of the store, it's not a sql-driven fn
  - get_mutant_test_counts too
  - get_target_stats could prob move out & replaced with a get_severity thing for just that sql stuff

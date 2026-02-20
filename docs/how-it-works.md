# How it works

[Back to README](../README.md)

Mewt is designed to provide as pleasant a developer experience as possible while conducting mutation campaigns, which are notoriously messy and slow.

## Campaign mechanics

Mewt operates on one single `mewt.sqlite` database. This stores the target files and mewt will reliably restore the original after a given mutation is tested, or after the campaign is interrupted with ctrl-c. However, this software is a work in progress so we strongly recommend running mutation campaigns against a clean git repo so that you can use `git reset --hard HEAD` to restore any mutations that escape the cleanup phase.

All target files are stored in the database and linked to a series of mutations. Each mutation is linked to one or zero outcomes. At the beginning of a mutation campaign, all targets are saved and all mutations are generated. This generally happens quickly, within a couple seconds.

Then, the real work begins: mewt will work through the list of target files, replacing it with a mutated version. For each mutated version, it will run the test command and save the outcome. If the mutation campaign is interrupted, it will pick up where it left off (unless the target file changed, in which case it will start over).

## Runtime expectations

This may take a very long time. Assuming the tests take 1 minute to run, there are 10 files, and 100 mutants were generated for each, the runtime (*assuming zero mewt overhead*) will be 1 * 10 * 100 = 1000 minutes or 16 hours.

## Skip optimization

For this reason, making mewt run fast is not enough to conduct fast mutation campaigns. Instead, a few features make this process somewhat less painful:
- **Resume by default:** if a campaign gets interrupted halfway through for whatever reason, we don't need to restart from the very beginning.
- **Customizable targets:** you can give mewt a directory as its target and it will mutate all supported files in this directory, which may take a long time. Or, you can give it one file and it will only mutate that file.
- **Skipping less severe mutants when more severe ones are uncaught:** if replacing an expression with a `throw` statement is not caught by the test suite, this indicates the expression is never run by the test suite. Therefore, it's safe to assume that any other mutation to this line will also not be caught by the test suite, so subsequent mutations are skipped. This can drastically decrease the runtime against poorly tested code. However, this also means the runtime will increase after the test suite is improved and the mutation campaign starts testing parts of the code more deeply than it did before.

Pass `--comprehensive` to `mewt run` to disable this optimization and test all mutants even when more severe ones on the same line are uncaught.

## When to run campaigns

Despite these features, mutation campaigns are best conducted infrequently, e.g. after an overhaul to the test suite rather than after adding each individual test. Therefore, mutation testing is not suitable for running in CI after every push. You may want to run a campaign at the end of the day so that it can run overnight.

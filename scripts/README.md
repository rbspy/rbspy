### How to add a new ruby version

You have a ruby version that is not supported and you want to raise a PR to support it. You have found the script that can help you with that!

### Dependencies

You will need to have forked/cloned this repo and have several things in order to use this script.

1. You will need clang/llvm
2. You will need https://github.com/rust-lang/rust-bindgen
3. You will need to have cloned ruby into `~/clones/ruby` from https://github.com/ruby/ruby and checked out the tag/branch you wish to build against
4. You will have to compile ruby to generate dynamic header files
5. You will need a compiled binary for the ruby version you are building first on your `$PATH`

### Executing the script

Once you have satisfied dependencies, you can run the script (NOTE: you must run the script from the root of the project) with a single argument that represents the version you are building (in this case 2.5.8):

```
./scripts/bindgen.sh 2_5_8
```

This should generate a file `ruby-structs/src/ruby_2_5_8.rs` with the updated bindings. Once you have this you can check it in as part of a PR to point to it (for example: https://github.com/rbspy/rbspy/pull/261)

### Running the script via Docker

There is also a Dockerfile that can be used to run bindgen for individual ruby versions.

Here is an example of how to run it:

```
docker build -t rbspy-bindgen -f scripts/Dockerfile .
docker run -v $(pwd)/ruby-structs:/opt/rbspy/ruby-structs rbspy-bindgen
```

You can patch the `RUBY_VERSION` in the Dockerfile to target different versions.

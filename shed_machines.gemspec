# frozen_string_literal: true

require_relative 'lib/shed_machines/version'

Gem::Specification.new do |spec|
  spec.name = 'shed_machines'
  spec.version = ShedMachines::VERSION
  spec.authors = ['Abdelkader Boudih']
  spec.email = ['oss@seuros.com']

  spec.summary = 'Adaptive concurrency limits and load shedding for Ruby'
  spec.description = 'AIMD and Vegas move a concurrency limit from what completed calls report, CoDel sheds ' \
                     'queued work by how long it waited, and a retry budget stops a client from amplifying ' \
                     'an outage. Bound to Ruby from the no_std shed-machines crate: the control loops only, ' \
                     'with the semaphore, the queue and the dice left to the caller.'
  spec.homepage = 'https://github.com/seuros/shed_machines'
  spec.license = 'MIT'
  spec.required_ruby_version = '>= 4.0.0'

  spec.metadata['source_code_uri'] = spec.homepage
  spec.metadata['changelog_uri'] = "#{spec.homepage}/blob/main/CHANGELOG.md"
  spec.metadata['bug_tracker_uri'] = "#{spec.homepage}/issues"
  spec.metadata['rubygems_mfa_required'] = 'true'
  spec.metadata['cargo_crate_name'] = 'shed_machines_native'
  spec.metadata['cargo_manifest_path'] = 'ext/shed_machines_native/ffi/Cargo.toml'

  spec.files = Dir.glob(%w[
                          lib/**/*.rb
                          sig/**/*.rbs
                          ext/**/*.{rb,rs,toml}
                          Cargo.toml
                          Cargo.lock
                          README.md
                          CHANGELOG.md
                          LICENSE.txt
                        ]).select { |path| File.exist?(path) }
                   .reject { |path| path =~ %r{/tests(/|\.rs\z)} }
  spec.require_paths = ['lib']
  spec.extensions = ['ext/shed_machines_native/extconf.rb']

  # A source install runs extconf.rb, which requires rb_sys/mkmf, and RubyGems
  # clears GEM_PATH while building the extension. Without this the build only
  # works where rb_sys happens to be installed globally.
  spec.add_dependency 'rb_sys', '~> 0.9'
end

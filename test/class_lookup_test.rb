# frozen_string_literal: true

require_relative 'test_helper'
require 'open3'
require 'rbconfig'

# The classes of the values the extension wraps (Aimd, Vegas, Codel, Budget)
# are fixed when it loads. Nothing a caller does later, redefining a constant
# or `Object.const_get`, reaches a value being built. Each case runs in a
# fresh process: the class is fixed once per process, at load.
class ClassLookupTest < Minitest::Test
  LIB = File.expand_path('../lib', __dir__)

  PRELUDE = <<~RUBY
    require 'shed_machines'
    S = ShedMachines
    BOUNDS = S::Bounds.new(1, 100)
    BACKOFF = S::Backoff.new(3, 4)
    BAND = S::Band.new(2, 4)
    SAMPLE = S::Sample.new(1_000_000, 10, :ok)
  RUBY

  # How each wrapped value is built from its class, and one of its own
  # methods used on it.
  BUILD = {
    Aimd: '.new(BOUNDS, 10, BACKOFF)',
    Vegas: '.new(BOUNDS, 10, BAND, 8)',
    Codel: '.default',
    Budget: '.default'
  }.freeze

  USE = {
    Aimd: '(value.observe(SAMPLE); value.limit)',
    Vegas: '(value.observe(SAMPLE); value.limit)',
    Codel: 'value.admit(0, 1)',
    Budget: 'value.requested'
  }.freeze

  USED = { Aimd: '11', Vegas: '11', Codel: 'admit', Budget: 'counted' }.freeze

  # Runs `body` after PRELUDE in a new Ruby; answers its last line of output.
  def isolated(body)
    out, err, status = Open3.capture3(RbConfig.ruby, '-I', LIB, '-e', PRELUDE + body)
    assert status.success?, "exited #{status.exitstatus}: #{err}"
    out.lines.last&.chomp
  end

  # Bug: a constant redefined before the first value made the value an
  # instance of the new class.
  BUILD.each do |name, build|
    define_method(:"test_a_redefined_#{name.downcase}_constant_does_not_change_the_value_class") do
      assert_equal "true #{USED[name]}", isolated(<<~RUBY)
        original = S::#{name}
        S.send(:remove_const, :#{name})
        S.const_set(:#{name}, Class.new)
        value = original#{build}
        puts [value.instance_of?(original), #{USE[name]}].join(' ')
      RUBY
    end
  end

  # Bug: building the first value ran `Object.const_get`, a caller's Ruby code
  # in the middle of a native call.
  def test_building_a_value_runs_no_const_get
    assert_equal '0 Aimd Vegas Codel Budget', isolated(<<~RUBY)
      hits = 0
      Object.define_singleton_method(:const_get) do |*args|
        hits += 1
        super(*args)
      end
      values = [#{BUILD.map { |name, build| "S::#{name}#{build}" }.join(', ')}]
      puts [hits, *values.map { |value| value.class.name.split('::').last }].join(' ')
    RUBY
  end

  # Bug: a redefined `Object.const_get` that raised hit an `unwrap` in the
  # extension: an uncatchable fatal, and the process exited.
  def test_a_raising_const_get_does_not_kill_the_process
    assert_equal USED.values.join(' '), isolated(<<~RUBY)
      Object.define_singleton_method(:const_get) { |*| raise IOError, 'lookup ran mid-call' }
      used = [#{BUILD.map { |name, build| "(value = S::#{name}#{build}; #{USE[name]})" }.join(', ')}]
      puts used.join(' ')
    RUBY
  end
end

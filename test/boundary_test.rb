# frozen_string_literal: true

require 'test_helper'

module ShedMachines
  # The native boundary is the authority: sealing stops accidents, and a value
  # forged past it is refused before it reaches a control law.
  class BoundaryTest < Minitest::Test
    include Samples

    VALUES = [Bounds, Sample, Backoff, Band].freeze

    # Ruby binds a named subclass's constant before `inherited` raises, so the
    # constant leaks; it is still refused wherever the binding reads it.
    module Forged; end

    def forge(value, name)
      return Forged.const_get(name, false) if Forged.const_defined?(name, false)

      assert_raises(TypeError) { Forged.module_eval("class #{name} < #{value}; end", __FILE__, __LINE__) }
      forged = Forged.const_get(name, false)
      forged.define_method(:initialize) { |**kw| Data.instance_method(:initialize).bind_call(self, **kw) }
      forged
    end

    def test_every_value_is_sealed_against_anonymous_and_named_subclasses
      VALUES.each do |value|
        assert_raises(TypeError) { Class.new(value) }
        forged = forge(value, "Sealed#{value.name.split('::').last}")

        assert_operator forged, :<, value, 'the constant leaks: Ruby binds it before inherited raises'
        assert_raises(NoMethodError) { value.allocate }
      end
    end

    def test_a_forged_subclass_value_is_a_type_error
      bounds = forge(Bounds, 'Bounds').new(min: 0, max: 0)
      sample = forge(Sample, 'Sample').new(rtt: 1, in_flight: 0, outcome: :ok)
      backoff = forge(Backoff, 'Backoff').new(kept: 9, of: 1)
      band = forge(Band, 'Band').new(alpha: 9, beta: 1)

      assert_raises(TypeError) { Aimd.new(bounds, 1, Backoff.default) }
      assert_raises(TypeError) { Aimd.new(Bounds.default, 10, backoff) }
      assert_raises(TypeError) { Vegas.new(Bounds.default, 10, band, 1) }
      aimd = Aimd.new(Bounds.new(1, 100), 10, Backoff.default)

      assert_raises(TypeError) { aimd.observe(sample) }
      assert_equal 10, aimd.limit
    end

    # A singleton class opened on a genuine value does not change what it is.
    def test_an_opened_singleton_class_is_still_the_gems_value
      bounds = Bounds.new(1, 100).tap(&:singleton_class)
      aimd = Aimd.new(bounds, 10, Backoff.default.tap(&:singleton_class))
      vegas = Vegas.new(bounds, 10, Band.default.tap(&:singleton_class), 4)
      aimd.observe(ok(4 * MS, 10).tap(&:singleton_class))

      assert_equal [11, 10], [aimd.limit, vegas.limit]
    end

    def test_a_marshal_edited_value_is_revalidated_into_a_range_error
      forged = Marshal.load(Marshal.dump(Bounds.new(1, 5)).sub("i\x06".b, "i\x00".b))

      assert_instance_of Bounds, forged
      assert_equal 0, forged.min
      assert_raises(RangeError) { Aimd.new(forged, 1, Backoff.default) }

      sample = Marshal.load(Marshal.dump(Sample.new(7, 1, :ok)).sub("i\x06".b, "i\x00".b))

      assert_equal 0, sample.in_flight
      aimd = Aimd.new(Bounds.new(1, 100), 10, Backoff.default)

      assert_raises(RangeError) { aimd.observe(sample) }
      assert_equal 10, aimd.limit
    end

    # A reader builds its Data answer with no borrow held, so a `new` that
    # re-enters the controller is an ordinary call, never an uncatchable fatal.
    def test_a_reader_whose_answer_re_enters_the_controller_is_not_fatal
      aimd = Aimd.new(Bounds.new(1, 100), 10, Backoff.default)
      grow = -> { aimd.observe(ok(4 * MS, aimd.limit)) }

      assert_equal Bounds.new(1, 100), reenter(Bounds, grow) { aimd.bounds }
      assert_equal Backoff.default, reenter(Backoff, grow) { aimd.backoff }
      assert_equal 12, aimd.limit

      vegas = Vegas.new(Bounds.new(1, 100), 10, Band.default, 4)
      shrink = -> { vegas.observe(overload(4 * MS, 10)) }

      assert_equal Bounds.new(1, 100), reenter(Bounds, shrink) { vegas.bounds }
      assert_equal Band.default, reenter(Band, shrink) { vegas.band }
      assert_equal 2, vegas.limit
    end

    # Fields are read from the struct itself: a redefined reader is never
    # called, so it can neither lie nor re-enter.
    def test_a_redefined_reader_is_ignored
      bounds = Bounds.new(1, 100)
      backoff = Backoff.default
      band = Band.default
      sample = ok(4 * MS, 10)
      readers = { Bounds => %i[min max], Backoff => %i[kept of], Band => %i[alpha beta],
                  Sample => %i[rtt in_flight outcome] }
      redefined(readers) do
        aimd = Aimd.new(bounds, 10, backoff)
        aimd.observe(sample)
        vegas = Vegas.new(bounds, 10, band, 4)

        assert_equal [11, 10], [aimd.limit, vegas.limit]
      end
    end

    def test_classes_held_by_the_binding_survive_gc_compaction
      aimd = Aimd.new(Bounds.new(1, 100), 10, Backoff.default)
      vegas = Vegas.new(Bounds.new(1, 200), 10, Band.default, Vegas::DEFAULT_BASELINE_WINDOW)
      # Compacting moves every unpinned object: a class the binding holds
      # unpinned would be a dangling reference and the next build would crash.
      GC.verify_compaction_references(expand_heap: true, toward: :empty)

      assert_equal [Bounds.new(1, 100), Backoff.default], [aimd.bounds, aimd.backoff]
      assert_equal Band.default, vegas.band
    end

    private

    # Runs the block with every listed reader raising if called.
    def redefined(readers)
      readers.each do |klass, names|
        names.each do |name|
          klass.alias_method(:"__real_#{name}", name)
          klass.define_method(name) { raise "#{name} reader called" }
        end
      end
      yield
    ensure
      readers.each do |klass, names|
        names.each do |name|
          klass.alias_method(name, :"__real_#{name}")
          klass.remove_method(:"__real_#{name}")
        end
      end
    end

    # Runs the block with `value.new` calling `side` first.
    def reenter(value, side)
      original = value.method(:new)
      value.define_singleton_method(:new) do |*args, **kw|
        side.call
        original.call(*args, **kw)
      end
      yield
    ensure
      value.define_singleton_method(:new, original)
    end
  end
end

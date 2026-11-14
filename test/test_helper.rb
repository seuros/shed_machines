# frozen_string_literal: true

$LOAD_PATH.unshift File.expand_path('../lib', __dir__)

require 'shed_machines'
require 'minitest/autorun'

module ShedMachines
  # Samples and spans for the tests, in Integer nanoseconds.
  module Samples
    MS = 1_000_000

    def ok(rtt, in_flight) = Sample.new(rtt, in_flight, :ok)

    def overload(rtt, in_flight) = Sample.new(rtt, in_flight, :overload)
  end
end

# frozen_string_literal: true

require_relative 'shed_machines/version'

# Aimd, Vegas, Codel and Budget are defined by the native extension; there is no
# pure-Ruby implementation to fall back to.
require 'shed_machines_native/shed_machines_native'

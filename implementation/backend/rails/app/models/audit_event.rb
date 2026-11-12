# frozen_string_literal: true

class AuditEvent < ApplicationRecord
  validates :action, presence: true
  validates :resource_type, presence: true
end

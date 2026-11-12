# frozen_string_literal: true

class Import < ApplicationRecord
  belongs_to :dataset
  belongs_to :project

  validates :status, inclusion: { in: %w[pending processing succeeded failed] }
  validates :source_type, presence: true
end

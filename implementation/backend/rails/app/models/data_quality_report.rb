# frozen_string_literal: true

class DataQualityReport < ApplicationRecord
  belongs_to :dataset
  belongs_to :dataset_version, optional: true
  belongs_to :import, optional: true

  validates :status, inclusion: { in: %w[pending completed failed] }
end

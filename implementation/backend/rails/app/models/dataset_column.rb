# frozen_string_literal: true

class DatasetColumn < ApplicationRecord
  belongs_to :dataset_schema

  validates :name, presence: true, uniqueness: { scope: :dataset_schema_id }
  validates :data_type, presence: true
end

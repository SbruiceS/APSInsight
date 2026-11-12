# frozen_string_literal: true

class DatasetSchema < ApplicationRecord
  belongs_to :dataset
  has_many :dataset_columns, dependent: :destroy

  validates :version, presence: true, uniqueness: { scope: :dataset_id }
end

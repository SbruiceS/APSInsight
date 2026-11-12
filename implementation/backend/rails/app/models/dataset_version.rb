# frozen_string_literal: true

class DatasetVersion < ApplicationRecord
  belongs_to :dataset

  validates :version_number, presence: true, uniqueness: { scope: :dataset_id }
  validates :checksum, presence: true
end

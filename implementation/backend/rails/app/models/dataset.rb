# frozen_string_literal: true

class Dataset < ApplicationRecord
  belongs_to :project
  belongs_to :organization
  has_many :dataset_versions, dependent: :destroy
  has_many :imports, dependent: :destroy
  has_many :dataset_schemas, dependent: :destroy
  has_many :data_quality_reports, dependent: :destroy
  has_many :soil_samples, dependent: :nullify
  has_many :observations, dependent: :nullify

  validates :name, presence: true
  validates :slug, presence: true, uniqueness: { scope: :project_id }
  validates :status, inclusion: { in: %w[draft active archived] }

  def current_version
    dataset_versions.order(version_number: :desc).first
  end

  def current_schema
    dataset_schemas.order(version: :desc).first
  end
end
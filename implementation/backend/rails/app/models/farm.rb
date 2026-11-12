# frozen_string_literal: true

class Farm < ApplicationRecord
  belongs_to :organization
  belongs_to :project, optional: true
  has_many :fields, dependent: :destroy

  validates :name, presence: true
  validates :slug, presence: true, uniqueness: { scope: :organization_id }
end

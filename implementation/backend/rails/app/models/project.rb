# frozen_string_literal: true

class Project < ApplicationRecord
  belongs_to :organization
  has_many :datasets, dependent: :destroy
  has_many :memberships, dependent: :destroy
  has_many :farms, dependent: :nullify

  validates :name, presence: true
  validates :slug, presence: true, uniqueness: { scope: :organization_id }
end
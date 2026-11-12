# frozen_string_literal: true

class Field < ApplicationRecord
  belongs_to :farm
  has_many :plots, dependent: :destroy
  has_many :soil_samples, dependent: :nullify

  validates :name, presence: true
end

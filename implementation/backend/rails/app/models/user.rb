# frozen_string_literal: true

class User < ApplicationRecord
  has_many :memberships, dependent: :destroy
  has_many :organizations, through: :memberships
  has_many :projects, through: :memberships

  validates :email, presence: true, uniqueness: true
  validates :name, presence: true
end

# frozen_string_literal: true

class CreateRoles < ActiveRecord::Migration[7.2]
  def change
    create_table :roles do |t|
      t.string :name, null: false
      t.string :key, null: false
      t.text :description
      t.timestamps
    end
    add_index :roles, :key, unique: true
  end
end
